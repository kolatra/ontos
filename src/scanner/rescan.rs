use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant},
};

use log::debug;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    database::DbConn,
    scanner::worker::ScanJob,
    util::misc::{wh_send, WHLog},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RescanStatus {
    Active,
    Idle,
}

impl Display for RescanStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RescanStatus::Active => write!(f, "Active"),
            RescanStatus::Idle => write!(f, "Idle"),
        }
    }
}

pub fn start_thread(status: Arc<Mutex<RescanStatus>>) {
    debug!("Starting rescan thread");
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60 * 60 * 5)).await;
            let status = status.lock().await;
            if status.eq(&RescanStatus::Idle) {
                continue;
            }

            wh_send(WHLog::Voyager, "Starting reping", Some("Voyager")).await;

            if let Err(e) = start_job().await {
                wh_send(
                    WHLog::Error,
                    &format!("Error starting job: {}", e),
                    Some("Voyager"),
                )
                .await;
            };
        }
    });
}

async fn start_job() -> anyhow::Result<()> {
    let list = db_list().await?;
    let scan = ScanJob {
        ips: list,
        timeout: Duration::from_secs(5),
        workers: 10,
    };

    let now = Instant::now();
    super::worker::run_blocking(scan).await;

    let msg = format!("Finished reping in {}s", now.elapsed().as_secs_f32());
    wh_send(WHLog::Voyager, &msg, Some("Voyager")).await;

    Ok(())
}

async fn db_list() -> anyhow::Result<Vec<String>> {
    let conn = DbConn::new().await?;
    let list = conn.get_all_ips(true).await?;

    Ok(list)
}
