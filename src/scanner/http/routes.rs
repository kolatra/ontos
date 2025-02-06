use axum::{
    extract,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::scanner::worker;
use crate::scanner::{rescan::RescanStatus, worker::ScanJob};

use super::AppState;

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScanInput {
    pub hosts: Vec<String>,
    pub timeout: Option<i32>,
}

pub fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/scan", post(single_scan))
        .route("/repings/:op", post(toggle_repings))
}

pub async fn index() -> Json<Response> {
    Json(Response {
        status: 200,
        message: "Hello, World!".to_string(),
    })
}

pub async fn single_scan(
    Extension(state): Extension<AppState>,
    extract::Json(input): extract::Json<ScanInput>,
) -> Json<Response> {
    let timeout_sec = input.timeout.unwrap_or(10);
    let _db = &state.db;
    let default = "127.0.0.1".to_string();
    let host = input.hosts.first().unwrap_or(&default);

    let Some(job) = ScanJob::new(vec![host.clone()], Some(timeout_sec), None) else {
        return super::error(format!("no target provided"));
    };

    if let Err(e) = worker::run(job).await {
        return super::error(format!("failed to run scan job for {}: {}", host, e));
    }

    super::success(format!("scanning {} with {}s timeout", host, timeout_sec))
}

pub async fn multi_scan(
    Extension(state): Extension<AppState>,
    extract::Json(input): extract::Json<ScanInput>,
) -> Json<Response> {
    let timeout_sec = input.timeout.unwrap_or(10);
    let len = input.hosts.len();
    let _db = &state.db;

    let Some(job) = ScanJob::new(input.hosts, Some(timeout_sec), None) else {
        return super::error(format!("no targets provided"));
    };

    if let Err(e) = worker::run(job).await {
        return super::error(format!("failed to run scan job: {}", e));
    }

    super::success(format!("started scanning {} hosts", len))
}

pub async fn toggle_repings(
    Extension(state): Extension<AppState>,
    extract::Path(input): extract::Path<String>,
) -> Json<Response> {
    enum Action {
        Status,
        Toggle,
    }

    let action = match input {
        s if s == "status" => Action::Status,
        s if s == "toggle" => Action::Toggle,
        _ => return super::error("invalid input".to_string()),
    };

    match action {
        Action::Status => {
            let status = state.rescan_active.lock().await;

            super::success(format!("rescanner is {}", *status))
        }

        Action::Toggle => {
            let mut status = state.rescan_active.lock().await;

            match *status {
                RescanStatus::Idle => *status = RescanStatus::Active,
                RescanStatus::Active => *status = RescanStatus::Idle,
            };

            super::success(format!("rescanner is now {status}"))
        }
    }
}
