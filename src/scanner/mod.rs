use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::Extension;
use log::{debug, warn};
use tokio::sync::Mutex;

use crate::util::misc;

use self::{http::AppState, rescan::RescanStatus};

mod http;
mod rescan;
mod worker;

pub async fn start() -> anyhow::Result<()> {
    misc::check_env(None);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        warn!("violently exiting");
        std::process::exit(0);
    });

    let state = AppState {
        db: crate::database::DbConn::new().await?,
        rescan_active: Arc::new(Mutex::new(RescanStatus::Idle)),
    };

    rescan::start_thread(Arc::clone(&state.rescan_active));

    let port = {
        let var = std::env::var("VOYAGER_PORT")?;
        var.parse::<u16>()?
    };
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);

    debug!("Listening for commands on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            http::routes::app()
                .layer(Extension(state))
                .into_make_service(),
        )
        .await
        .unwrap();

    Ok(())
}
