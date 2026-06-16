use serenity::prelude::TypeMapKey;
use sqlx::PgPool;
use std::sync::Arc;

use crate::asset_manager::AssetManager;

pub struct BotState {
    pub pool: PgPool,
    pub guild_cache: Arc<crate::cache::GuildCache>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub asset_manager: Arc<AssetManager>,
}

pub struct BotStateKey;
impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
