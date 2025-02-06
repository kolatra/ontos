#![allow(dead_code)]
use std::sync::Arc;

use self::routes::Response;
use axum::Json;
use tokio::sync::Mutex;

use super::rescan::RescanStatus;

pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub db: crate::database::DbConn,
    pub rescan_active: Arc<Mutex<RescanStatus>>,
}

pub fn success(message: String) -> Json<Response> {
    Json(Response {
        status: 200,
        message,
    })
}

pub fn error(message: String) -> Json<Response> {
    Json(Response {
        status: 400,
        message,
    })
}
