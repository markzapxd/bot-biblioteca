use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct SendMessage {
    pub channel_id: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct CreateRole {
    pub guild_id: String,
    pub name: String,
    pub color: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub guilds: usize,
    pub ping: u64,
    pub users: String,
    pub uptime: String,
}

#[derive(Serialize)]
pub struct GuildInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Serialize)]
pub struct MessageInfo {
    pub author: String,
    pub content: String,
    pub timestamp: i64,
    pub bot: bool,
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}
