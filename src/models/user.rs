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
    pub nickname_history: serde_json::Value,
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
    #[serde(default)]
    pub guild_id: Option<String>,
    pub url: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicknameEntry {
    #[serde(default)]
    pub guild_id: Option<String>,
    pub name: String,
    pub date: DateTime<Utc>,
}

impl User {
    pub fn get_username_history(&self) -> Vec<UsernameEntry> {
        serde_json::from_value(self.username_history.clone()).unwrap_or_default()
    }

    pub fn get_avatar_history(&self) -> Vec<AvatarEntry> {
        serde_json::from_value(self.avatar_history.clone()).unwrap_or_default()
    }

    pub fn get_nickname_history(&self) -> Vec<NicknameEntry> {
        serde_json::from_value(self.nickname_history.clone()).unwrap_or_default()
    }

    pub fn is_private_mode(&self) -> bool {
        self.is_private.unwrap_or(false)
    }
}

#[cfg(test)]
mod avatar_privacy_tests {
    #[test]
    fn avatar_provenance_survives_deserialization_and_legacy_json() {
        let entries: Vec<super::AvatarEntry> = serde_json::from_value(serde_json::json!([
            {"url":"global", "date":"2026-01-01T00:00:00Z"},
            {"url":"private-a", "date":"2026-01-01T00:00:00Z", "guild_id":"1"}
        ]))
        .unwrap();
        assert_eq!(serde_json::to_value(&entries).unwrap()[1]["guild_id"], "1");
        assert_eq!(entries.len(), 2);
    }
}
