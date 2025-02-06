use std::time::{Duration, Instant};

use crate::util::types::{Description, Entry, Favicon, OntosPlayer};
use crate::{
    database::entities::{players, prelude::*},
    util::types::Server,
};
use anyhow::anyhow;
use log::{debug, error};
use rand::seq::SliceRandom;
use sea_orm::{
    prelude::*, Condition, ConnectOptions, Database, DatabaseTransaction, TransactionTrait,
};

use self::entities::{ips, servers};

pub mod entities;

#[derive(Clone, Debug)]
pub struct DbConn {
    pub client: DatabaseConnection,
}

#[derive(Clone, Debug)]
pub struct QueryParams {
    pub column: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct DbStats {
    pub ips: u64,
    pub servers: u64,
    pub players: u64,
}

impl QueryParams {
    fn to_column(&self) -> anyhow::Result<servers::Column> {
        let c = match self.column.as_str() {
            "id" => servers::Column::Id,
            "ip" => servers::Column::Ip,
            "version" => servers::Column::Version,
            "protocol" => servers::Column::Protocol,
            "max_players" => servers::Column::MaxPlayers,
            "online_players" => servers::Column::OnlinePlayers,
            "auth" => servers::Column::Auth,
            _ => return Err(anyhow!("Invalid column")),
        };

        Ok(c)
    }
}

impl DbConn {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: connect().await?,
        })
    }

    pub async fn add_server(&self, entry: Entry) -> anyhow::Result<DatabaseTransaction> {
        let Entry {
            server,
            description,
            favicon,
        } = entry;

        let now = Instant::now();
        let client = &self.client;
        let txn = client.begin().await?;

        let server_id = {
            server.update_scan_time(&txn).await?;
            server.insert(&txn).await?
        };

        description.insert(&txn, server_id).await?;

        if let Some(players) = server.sample_players {
            // !! find out why some players are null !!
            // TODO: try to find a server with null players to debug
            let list = OntosPlayer::from_sample(players, server_id);
            if let Err(e) = OntosPlayer::insert(&txn, list).await {
                error!("Failed to insert players: {}", e);
            };
        }

        if favicon.png.is_some() {
            favicon.insert(&txn, server_id).await?;
        }

        let elapsed = Instant::now() - now;
        debug!("Added server in {}ms", elapsed.as_millis());

        Ok(txn)
    }

    pub async fn create_stats(&self) -> anyhow::Result<DbStats> {
        let client = &self.client;

        let ips = ips::Entity::find().count(client).await?;
        let servers = servers::Entity::find().count(client).await?;
        let players = players::Entity::find().count(client).await?;

        Ok(DbStats {
            ips,
            servers,
            players,
        })
    }

    pub async fn get_random_ip(&self) -> anyhow::Result<String> {
        let client = &self.client;

        let size = Ips::find().count(client).await? as i32;
        let index = rand::random::<i32>() % size;
        let res = Ips::find_by_id(index).one(client).await?.unwrap();
        let host = format!("{}:{}", res.ip, res.port as u16);

        Ok(host)
    }

    pub async fn get_all_ips(&self, reping: bool) -> anyhow::Result<Vec<String>> {
        let client = &self.client;
        let mut ips = Ips::find().all(client).await?;

        if reping {
            ips.sort_by(|a, b| a.last_scanned.cmp(&b.last_scanned));
            ips.drain(..10_000);
        }

        let ips = ips
            .into_iter()
            .map(|ip| format!("{}:{}", ip.ip, ip.port))
            .collect();

        Ok(ips)
    }

    pub async fn get_some_ips(&self, amount: usize) -> anyhow::Result<Vec<String>> {
        let client = &self.client;
        let mut ips = Ips::find().all(client).await?;

        ips.shuffle(&mut rand::thread_rng());
        ips.truncate(amount);

        let ips = ips
            .into_iter()
            .map(|ip| format!("{}:{}", ip.ip, ip.port))
            .collect();

        Ok(ips)
    }

    pub async fn get_servers(&self, params: QueryParams) -> anyhow::Result<Vec<Entry>> {
        let client = &self.client;
        let column = params.to_column()?;
        let value = params.value;

        let mut condition = Condition::all();
        if let Ok(i) = value.parse::<i32>() {
            condition = condition.add(column.eq(i));
        } else {
            let value = format!("%{}%", value);
            condition = condition.add(column.like(&value));
        }

        let results = Servers::find().filter(condition).all(client).await?;
        let mut output = Vec::new();

        for model in results {
            output.push(Entry {
                server: Server::from_model(model.clone()),
                description: {
                    let model = model.find_related(Descriptions).one(client).await?;
                    model.map_or(Description::empty(), Description::from_model)
                },
                favicon: {
                    let model = model.find_related(Favicons).one(client).await?;
                    model.map_or(Favicon::empty(), Favicon::from_model)
                },
            });
        }

        Ok(output)
    }
}

async fn connect() -> anyhow::Result<DatabaseConnection> {
    let client_uri = std::env::var("DATABASE_URL")?;
    let mut opt = ConnectOptions::new(client_uri);
    opt.max_connections(1000);
    opt.idle_timeout(Duration::from_secs(8));
    opt.max_lifetime(Duration::from_secs(8));
    let client = Database::connect(opt).await?;

    Ok(client)
}
