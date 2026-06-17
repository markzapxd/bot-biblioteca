use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomCard {
    pub id: i32,
    pub guild_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub color: Option<String>,
    pub footer: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CustomCard {
    pub fn parse_color(&self) -> u32 {
        self.color
            .as_ref()
            .and_then(|c| {
                let trimmed = c.trim();
                let hex = if trimmed.starts_with('#') {
                    &trimmed[1..]
                } else {
                    trimmed
                };
                u32::from_str_radix(hex, 16).ok()
            })
            .unwrap_or(0x2B2D31)
    }
}
