use std::time::Duration;

use log::{error, info};
use tokio::task;

use crate::{
    util::types::{Entry, OntosAddress},
    web::server::{Response, WebRequest},
};

pub struct ScanJob {
    pub ips: Vec<String>,
    pub timeout: Duration,
    pub workers: usize,
}

impl ScanJob {
    pub fn new(ips: Vec<String>, timeout: Option<i32>, workers: Option<usize>) -> Option<Self> {
        if ips.is_empty() {
            return None;
        }

        Some(Self {
            ips,
            timeout: Duration::from_secs(timeout.unwrap_or(10) as u64),
            workers: workers.unwrap_or(1),
        })
    }
}

pub async fn run_blocking(job: ScanJob) {
    let ips = job.ips.clone();
    let timeout = job.timeout;
    let workers = job.workers;
    let len = ips.len();
    let mut chunks = ips.chunks((len / workers).max(1));
    let mut futures = Vec::new();

    info!("Scanning {} chunks with {} workers", chunks.len(), workers);
    for _ in 0..=workers {
        let Some(list) = chunks.next() else { break };
        let mut ips = list.to_vec();

        futures.push(tokio::spawn(
            async move { ping_slice(&mut ips, timeout).await },
        ));
    }

    futures::future::join_all(futures).await;
}

pub async fn run(job: ScanJob) -> anyhow::Result<()> {
    task::spawn(async move { run_blocking(job).await }).await?;

    Ok(())
}

async fn ping_slice(list: &mut Vec<String>, timeout: Duration) {
    let mut queue = vec![];
    loop {
        let Some(host) = list.pop() else { break };

        let ontos_addr = OntosAddress { host };

        let scan = match ontos_addr.ping_server(timeout).await {
            Ok(scan) => scan,
            Err(e) => {
                error!("Error scanning {}: {}", ontos_addr.host, e);
                continue;
            }
        };
        queue.push(scan);

        if queue.len() >= 10 {
            if let Err(e) = upload_servers(&queue).await {
                error!("Error uploading servers: {}", e);
            };
            queue.clear();
        }
    }
}

async fn upload_servers(queue: &Vec<Entry>) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = std::env::var("WEBSERVER_URL")?;
    let port = std::env::var("WEBSERVER_PORT")?;

    let input = WebRequest {
        servers: Some(queue.clone()), // kind of an expensive clone, but I don't want to prematurely optimize
        ..Default::default()
    };

    let res = client
        .post(format!("http://{}:{}/upload", url, port))
        .json(&input)
        .send()
        .await?
        .json::<Response>()
        .await?;

    match res.status {
        200 => info!("Uploaded {} servers", queue.len()),
        _ => error!("Failed to upload servers: {}", res.status),
    }

    Ok(())
}
