use serenity::prelude::TypeMapKey;
use sqlx::PgPool;
use std::sync::Arc;

pub struct BotState {
    pub pool: PgPool,
    pub guild_cache: Arc<crate::cache::GuildCache>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

pub struct BotStateKey;
impl TypeMapKey for BotStateKey {
    type Value = Arc<BotState>;
}
