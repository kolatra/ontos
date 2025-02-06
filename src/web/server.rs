#![allow(dead_code)]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use log::error;
use sea_orm::DatabaseTransaction;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    database::{DbConn, DbStats},
    util::types::Entry,
};

#[derive(Clone, Debug)]
struct AppState {
    database: DbConn,
    txn_queue: Arc<Mutex<Vec<DatabaseTransaction>>>,
    stats: Arc<Mutex<DbStats>>,
}

pub async fn start() -> anyhow::Result<()> {
    crate::util::misc::check_env(None);

    let conn = crate::database::DbConn::new().await?;

    let stats = conn.create_stats().await?;

    let state = AppState {
        database: conn,
        txn_queue: Arc::new(Mutex::new(Vec::new())),
        stats: Arc::new(Mutex::new(stats)),
    };

    let queue = Arc::clone(&state.txn_queue);
    tokio::task::spawn(async move {
        loop {
            let mut queue_copy = Vec::new();
            {
                let mut queue = queue.lock().await;
                if queue.is_empty() {
                    continue;
                }

                std::mem::swap(&mut queue_copy, &mut *queue);
            }

            for txn in queue_copy {
                if let Err(e) = txn.commit().await {
                    error!("Error committing transaction: {}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    let port = {
        let var = std::env::var("WEBSERVER_PORT")?;
        var.parse::<u16>()?
    };
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    dbg!(&addr);

    axum::Server::bind(&addr)
        .serve(app().layer(Extension(state)).into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/servers", get(get_server))
        .route("/upload", post(upload_servers))
}

// ! Remember this on return types for routes
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WebRequest {
    // get_server
    pub server_id: Option<u64>,

    // upload_servers
    pub servers: Option<Vec<Entry>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Entry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<Stats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub status: String,
    pub runtime_mode: String,
    pub stored_ips: u64,
    pub stored_servers: u64,
    pub stored_players: u64,
}

async fn index(Extension(state): Extension<AppState>) -> Json<Response> {
    let debug = crate::is_debug();
    let runtime_mode = if debug {
        "debug".to_string()
    } else {
        "release".to_string()
    };

    let stats = state.stats.lock().await;

    let data = ResponseData {
        results: None,
        stats: Some(Stats {
            status: "ok".to_string(),
            runtime_mode,
            stored_ips: stats.ips,
            stored_servers: stats.servers,
            stored_players: stats.players,
        }),
    };

    success(None, Some(data))
}

async fn get_server(
    Extension(state): Extension<AppState>,
    args: Json<WebRequest>,
) -> Json<Response> {
    let total = state.stats.lock().await;
    let servers = total.servers;
    let server_id = args.server_id.unwrap_or(rand::random::<u64>() % servers);
    let db = state.database;

    let params = crate::database::QueryParams {
        column: "id".to_string(),
        value: server_id.to_string(),
    };

    let Ok(entry) = db.get_servers(params).await else {
        return error("Internal server error");
    };

    let data = ResponseData {
        results: Some(entry),
        stats: None,
    };

    success(None, Some(data))
}

async fn upload_servers(
    Extension(state): Extension<AppState>,
    mut args: Json<WebRequest>,
) -> Json<Response> {
    let Some(servers) = args.servers.take() else {
        return error("No servers provided");
    };

    let Ok(conn) = DbConn::new().await else {
        return error("Internal server error");
    };

    for s in servers {
        let txn = match conn.add_server(s).await {
            Ok(txn) => txn,
            Err(e) => {
                error!("Error adding server: {}", e);
                continue;
            }
        };

        state.txn_queue.lock().await.push(txn);
    }

    if let Err(e) = update_stats(Extension(state)).await {
        error!("Error updating stats: {}", e);
    };

    success(None, None)
}

fn success(msg: Option<&str>, data: Option<ResponseData>) -> Json<Response> {
    Json(Response {
        status: 200,
        message: msg.map(|s| s.to_string()).unwrap_or("success".to_string()),
        data,
    })
}

fn error(msg: &str) -> Json<Response> {
    Json(Response {
        status: 400,
        message: msg.to_string(),
        data: None,
    })
}

async fn update_stats(Extension(state): Extension<AppState>) -> anyhow::Result<()> {
    let db = state.database;
    let new_stats = match db.create_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            return Err(anyhow::anyhow!("db error: {}", e));
        }
    };

    let mut stats = state.stats.lock().await;
    stats.ips = new_stats.ips;
    stats.servers = new_stats.servers;
    stats.players = new_stats.players;

    Ok(())
}
