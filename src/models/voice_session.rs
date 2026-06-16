use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VoiceSession {
    pub id: i32,
    pub user_id: String,
    pub guild_id: String,
    pub guild_name: Option<String>,
    pub channel_id: String,
    pub channel_name: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub duration: Option<i64>,
    pub members_at_end: serde_json::Value,
    pub active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VoiceSession {
    pub fn is_active(&self) -> bool {
        self.active.unwrap_or(false)
    }

    pub fn get_members_at_end(&self) -> Vec<String> {
        serde_json::from_value(self.members_at_end.clone()).unwrap_or_default()
    }

    pub fn duration_formatted(&self) -> String {
        let ms = self.duration.unwrap_or(0);
        let total_seconds = ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        let mut parts = Vec::new();
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{}s", seconds));
        }
        parts.join(" ")
    }
}
