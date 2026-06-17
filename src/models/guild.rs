use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Guild {
    pub guild_id: String,
    pub prefix: Option<String>,
    pub member_role_id: Option<String>,
    pub staff_channel_id: Option<String>,
    pub welcome_channel_id: Option<String>,
    pub log_channel_id: Option<String>,
    pub admin_role_id: Option<String>,
    pub staff_role_id: Option<String>,
    pub ticket_category_id: Option<String>,
    pub frin_monitor_channel_id: Option<String>,
    pub modules: serde_json::Value,
    pub webhook_url: Option<String>,
    pub premium: Option<bool>,
    pub track_mute: Option<bool>,
    pub track_deaf: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildModules {
    #[serde(default = "default_true")]
    pub antiraid: bool,
    #[serde(default = "default_true")]
    pub logs: bool,
    #[serde(default = "default_true")]
    pub tickets: bool,
    #[serde(default = "default_true")]
    pub voice_tracking: bool,
    #[serde(default = "default_true")]
    pub member_verification: bool,
    #[serde(default = "default_true")]
    pub log_calls: bool,
    #[serde(default = "default_true")]
    pub log_joins_leaves: bool,
    #[serde(default = "default_true")]
    pub log_roles: bool,
    #[serde(default = "default_true")]
    pub log_messages: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GuildModules {
    fn default() -> Self {
        Self {
            antiraid: true,
            logs: true,
            tickets: true,
            voice_tracking: false,
            member_verification: false,
            log_calls: true,
            log_joins_leaves: true,
            log_roles: true,
            log_messages: true,
        }
    }
}

impl Guild {
    pub fn get_modules(&self) -> GuildModules {
        serde_json::from_value(self.modules.clone()).unwrap_or_default()
    }

    pub fn is_module_enabled(&self, module: &str) -> bool {
        let modules = self.get_modules();
        match module {
            "antiraid" => modules.antiraid,
            "logs" => modules.logs,
            "tickets" => modules.tickets,
            "voice_tracking" => modules.voice_tracking,
            "member_verification" => modules.member_verification,
            "log_calls" => modules.log_calls,
            "log_joins_leaves" => modules.log_joins_leaves,
            "log_roles" => modules.log_roles,
            "log_messages" => modules.log_messages,
            _ => false,
        }
    }
}
