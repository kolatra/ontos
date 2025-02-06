use std::time::Duration;

use azalea_protocol::ServerAddress;
use base64::{engine::general_purpose, Engine};
use chrono::NaiveDateTime;
use craftping::tokio::ping;
use craftping::Response as CraftpingResponse;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, DatabaseTransaction, EntityTrait};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use crate::database::entities::descriptions::ActiveModel as DescModel;
use crate::database::entities::favicons::ActiveModel as FaviconModel;
use crate::database::entities::players::ActiveModel as PlayerModel;
use crate::database::entities::servers::ActiveModel as ServerModel;
use crate::database::entities::{descriptions, favicons, ips, players, servers};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OntosAddress {
    pub host: String,
}

impl OntosAddress {
    pub async fn ping_server(&self, timeout: Duration) -> anyhow::Result<Entry> {
        let scan = tokio::time::timeout(timeout, self.send_request()).await?;
        let packet = scan?;

        let ip = ServerAddress::try_from(self.host.as_str()).unwrap();
        let entry = Entry::new(packet, ip);
        Ok(entry)
    }

    async fn send_request(&self) -> anyhow::Result<CraftpingResponse> {
        let Ok(addr) = ServerAddress::try_from(self.host.as_str()) else {
            return Err(anyhow::anyhow!(""))
        };
        let host = addr.host;
        let port = addr.port;

        let mut stream = TcpStream::connect((host.clone(), port)).await?;
        let response = ping(&mut stream, &host, port).await?;
        Ok(response)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub server: Server,
    pub description: Description,
    pub favicon: Favicon,
}

impl Entry {
    pub fn new(packet: CraftpingResponse, addr: ServerAddress) -> Self {
        Self {
            server: Server {
                id: 0,
                ip: addr.host,
                port: addr.port,
                version: packet.version,
                protocol: packet.protocol,
                max_players: packet.max_players,
                online_players: packet.online_players,
                sample_players: {
                    // trying to avoid a null player issue
                    match packet.sample {
                        Some(players) => {
                            let db_players = players
                                .iter()
                                .map(|player| OntosPlayer {
                                    name: player.name.clone(),
                                    uuid: uuid::Uuid::parse_str(&player.id).unwrap(),
                                    last_seen: chrono::Utc::now().naive_utc(),
                                    server_id: 0,
                                })
                                .collect();

                            Some(db_players)
                        }
                        _ => None,
                    }
                },
                auth: OnlineStatus::Online,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                forge: packet.forge_data.is_some(),
            },

            description: Description {
                id: 0,
                server_id: 0,
                text: packet.description.text,
                bold: packet.description.bold,
                italic: packet.description.italic,
                underline: packet.description.underlined,
                strikethrough: packet.description.strikethrough,
                obfuscated: packet.description.obfuscated,
                colour: packet.description.color.unwrap_or("white".to_string()),
            },

            favicon: Favicon {
                id: 0,
                png: packet
                    .favicon
                    .map(|favicon| general_purpose::STANDARD_NO_PAD.encode(favicon)),
                server_id: 0,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: i32,
    pub ip: String,
    pub port: u16,
    pub version: String,
    pub protocol: i32,
    pub max_players: usize,
    pub online_players: usize,
    pub sample_players: Option<Vec<OntosPlayer>>,
    pub auth: OnlineStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub forge: bool,
}

impl Server {
    pub fn model(&self) -> ServerModel {
        ServerModel {
            ip: ActiveValue::Set(self.ip.clone()),
            port: ActiveValue::Set(self.port as i32),
            version: ActiveValue::Set(self.version.clone()),
            protocol: ActiveValue::Set(self.protocol),
            max_players: ActiveValue::Set(self.max_players as i32),
            online_players: ActiveValue::Set(self.online_players as i32),
            auth: ActiveValue::Set(self.auth.to_string_but_consistent()),
            forge: ActiveValue::Set(self.forge),
            created_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
    }

    pub fn from_model(model: servers::Model) -> Self {
        Self {
            id: model.id,
            ip: model.ip,
            port: model.port as u16,
            version: model.version,
            protocol: model.protocol,
            max_players: model.max_players as usize,
            online_players: model.online_players as usize,
            sample_players: None,
            auth: match model.auth.as_str() {
                "Online" => OnlineStatus::Online,
                "Offline" => OnlineStatus::Offline,
                _ => OnlineStatus::Offline,
            },
            created_at: model.created_at,
            updated_at: model.updated_at,
            forge: model.forge,
        }
    }

    pub async fn insert<T: sea_orm::ConnectionTrait>(&self, txn: &T) -> anyhow::Result<i32> {
        let id = servers::Entity::insert(self.model())
            .on_conflict(
                OnConflict::columns(vec![servers::Column::Ip, servers::Column::Port])
                    .update_columns(vec![
                        servers::Column::Version,
                        servers::Column::Protocol,
                        servers::Column::MaxPlayers,
                        servers::Column::OnlinePlayers,
                        servers::Column::Auth,
                        servers::Column::Forge,
                        servers::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(txn)
            .await?
            .last_insert_id;

        Ok(id)
    }

    pub async fn update_scan_time(&self, txn: &DatabaseTransaction) -> anyhow::Result<()> {
        ips::Entity::insert(ips::ActiveModel {
            ip: ActiveValue::Set(self.ip.clone()),
            port: ActiveValue::Set(self.port as i32),
            last_scanned: ActiveValue::Set(Some(chrono::Utc::now().naive_utc())),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns(vec![ips::Column::Ip, ips::Column::Port])
                .update_column(ips::Column::LastScanned)
                .to_owned(),
        )
        .exec(txn)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Description {
    pub id: i32,
    pub server_id: i32,
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    pub colour: String,
}

impl Description {
    pub fn model(self, server_id: i32) -> DescModel {
        DescModel {
            server_id: ActiveValue::Set(server_id),
            text: ActiveValue::Set(self.text),
            bold: ActiveValue::Set(false),
            italic: ActiveValue::Set(false),
            underline: ActiveValue::Set(false),
            strikethrough: ActiveValue::Set(false),
            obfuscated: ActiveValue::Set(false),
            colour: ActiveValue::Set("false".to_string()),
            ..Default::default()
        }
    }

    pub fn from_model(model: descriptions::Model) -> Self {
        Self {
            id: model.id,
            server_id: model.server_id,
            text: model.text,
            bold: model.bold,
            italic: model.italic,
            underline: model.underline,
            strikethrough: model.strikethrough,
            obfuscated: model.obfuscated,
            colour: model.colour,
        }
    }

    pub fn empty() -> Self {
        Self {
            id: 0,
            server_id: 0,
            text: "".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            obfuscated: false,
            colour: "white".to_string(),
        }
    }

    pub async fn insert<T: sea_orm::ConnectionTrait>(
        self,
        txn: &T,
        server_id: i32,
    ) -> anyhow::Result<()> {
        descriptions::Entity::insert(self.model(server_id))
            .on_conflict(
                OnConflict::columns(vec![
                    descriptions::Column::ServerId,
                    descriptions::Column::Text,
                ])
                .update_columns(vec![
                    descriptions::Column::Text,
                    descriptions::Column::Bold,
                    descriptions::Column::Italic,
                    descriptions::Column::Underline,
                    descriptions::Column::Strikethrough,
                    descriptions::Column::Obfuscated,
                    descriptions::Column::Colour,
                ])
                .to_owned(),
            )
            .exec(txn)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OntosPlayer {
    pub name: String,
    pub uuid: uuid::Uuid,
    pub last_seen: chrono::NaiveDateTime,
    pub server_id: i32,
}

impl OntosPlayer {
    pub fn from_sample(list: Vec<Self>, server_id: i32) -> Vec<PlayerModel> {
        list.into_iter()
            .map(|player| PlayerModel {
                name: ActiveValue::Set(player.name),
                uuid: ActiveValue::Set(player.uuid.to_string()),
                last_seen: ActiveValue::Set(chrono::Utc::now().naive_utc()),
                server_id: ActiveValue::Set(server_id),
                ..Default::default()
            })
            .collect()
    }

    pub async fn insert<T: sea_orm::ConnectionTrait>(
        txn: &T,
        list: Vec<PlayerModel>,
    ) -> anyhow::Result<()> {
        players::Entity::insert_many(list)
            .on_conflict(
                OnConflict::columns(vec![players::Column::ServerId, players::Column::Uuid])
                    .update_column(players::Column::LastSeen)
                    .to_owned(),
            )
            .exec(txn)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Favicon {
    pub id: i32,
    pub png: Option<String>,
    pub server_id: i32,
}

impl Favicon {
    pub fn model(self, server_id: i32) -> FaviconModel {
        FaviconModel {
            server_id: ActiveValue::Set(server_id),
            png: ActiveValue::Set(self.png.unwrap()),
            ..Default::default()
        }
    }

    pub fn from_model(model: favicons::Model) -> Self {
        Self {
            id: model.id,
            png: Some(model.png),
            server_id: model.server_id,
        }
    }

    pub fn empty() -> Self {
        Self {
            id: 0,
            png: None,
            server_id: 0,
        }
    }

    pub async fn insert<T: sea_orm::ConnectionTrait>(
        self,
        txn: &T,
        server_id: i32,
    ) -> anyhow::Result<()> {
        favicons::Entity::insert(self.model(server_id))
            .on_conflict(
                OnConflict::column(favicons::Column::ServerId)
                    .update_columns(vec![favicons::Column::Png])
                    .to_owned(),
            )
            .exec(txn)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OnlineStatus {
    Online,
    Offline,
}

impl OnlineStatus {
    pub fn to_string_but_consistent(&self) -> String {
        match self {
            OnlineStatus::Online => "Online".to_string(),
            OnlineStatus::Offline => "Offline".to_string(),
        }
    }
}
