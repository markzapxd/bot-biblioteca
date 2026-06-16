use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: String,
    pub is_private: Option<bool>,
    pub total_voice_time: Option<i64>,
    pub premium: Option<bool>,
    pub username_history: serde_json::Value,
    pub avatar_history: serde_json::Value,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernameEntry {
    pub name: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarEntry {
    pub url: String,
    pub date: DateTime<Utc>,
}

impl User {
    pub fn get_username_history(&self) -> Vec<UsernameEntry> {
        serde_json::from_value(self.username_history.clone()).unwrap_or_default()
    }

    pub fn get_avatar_history(&self) -> Vec<AvatarEntry> {
        serde_json::from_value(self.avatar_history.clone()).unwrap_or_default()
    }

    pub fn is_private_mode(&self) -> bool {
        self.is_private.unwrap_or(false)
    }
}
