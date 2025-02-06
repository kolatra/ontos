use std::fmt::{Display, Formatter};

use base64::{engine::general_purpose, Engine};
use log::debug;
use serde_json::Value;
use serenity::{http::Http, model::webhook::Webhook};

use crate::util::logs::init_logs;

const KEYS: [&str; 12] = [
    "API_KEY",
    "ADMIN_KEY",
    "DATABASE_URL",
    "ERROR_WEBHOOK_URL",
    "INFO_WEBHOOK_URL",
    "VOYAGER_WEBHOOK_URL",
    "IMGUR_CLIENT_ID",
    "LOG_LEVEL",
    "VOYAGER_ADDR",
    "VOYAGER_PORT",
    "WEBSERVER_URL",
    "WEBSERVER_PORT",
];

pub const DEFAULT_IMAGE: &str = "https://i.imgur.com/n4BlJtE.png";

pub fn strip_ansi(input: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[.*?m").unwrap();
    re.replace_all(input, "").to_string()
}

pub fn check_env(extra_keys: Option<Vec<&str>>) {
    let mut extra_keys = extra_keys.unwrap_or_default();
    KEYS.to_vec().append(&mut extra_keys);

    let bad_keys = KEYS
        .into_iter()
        .filter(|key| std::env::var(key).is_err())
        .collect::<Vec<_>>();

    if !bad_keys.is_empty() {
        panic!("missing env vars: {:?}", bad_keys);
    }

    init_logs().unwrap();

    debug!("env vars are set");
}

/// Bytes should be decoded base64 with the png prefix removed
pub async fn upload_image(bytes: Vec<u8>) -> anyhow::Result<String> {
    let image = general_purpose::STANDARD.encode(bytes);
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.imgur.com/3/image")
        .header(
            "Authorization",
            format!("Client-ID {}", std::env::var("IMGUR_CLIENT_ID")?),
        )
        .form(&[("image", image)])
        .send()
        .await?
        .text()
        .await?;

    let mut ret_url = DEFAULT_IMAGE.to_string();
    let res: Value = serde_json::from_str(&res)?;
    if let Some(url) = res.get("data").unwrap().get("link") {
        ret_url = url.to_string().replace('\"', "");
    }

    Ok(ret_url)
}

pub fn decode_favicon(favicon: String) -> Vec<u8> {
    let favicon = favicon.replace("data:image/png;base64,", "");
    let bytes = favicon.into_bytes();

    match general_purpose::STANDARD.decode(bytes) {
        Ok(bytes) => bytes,
        Err(_) => vec![],
    }
}

pub enum WHLog {
    Info,
    Error,
    Voyager,
}

impl Display for WHLog {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WHLog::Info => write!(f, "INFO"),
            WHLog::Error => write!(f, "ERROR"),
            WHLog::Voyager => write!(f, "VOYAGER"),
        }
    }
}

pub async fn wh_send(level: WHLog, msg: &str, username: Option<&str>) {
    let http = Http::new("");
    let username = username.unwrap_or("logger");
    let url = std::env::var(format!("{}_WEBHOOK_URL", level)).unwrap();
    let webhook = match Webhook::from_url(&http, &url).await {
        Ok(webhook) => webhook,
        Err(e) => {
            log::error!("Could not create webhook {}", e);
            return;
        }
    };

    webhook
        .execute(&http, false, |w| w.content(msg).username(username))
        .await
        .expect("Could not execute webhook.");
}
