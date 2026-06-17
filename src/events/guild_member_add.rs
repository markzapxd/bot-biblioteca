use serenity::all::{Context, Member};

pub async fn handle(ctx: Context, member: Member) {
    tracing::info!("Member joined: {} ({})", member.user.name, member.user.id);
    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = &state.pool;
        let guild_id = member.guild_id.get();
        let guild_id_str = guild_id.to_string();

        let guild_config = if let Some(config) = state.guild_cache.get(&guild_id_str) {
            config
        } else {
            let config = match crate::repositories::guild_repo::find_by_id(pool, &guild_id_str).await {
                Ok(Some(c)) => c,
                _ => {
                    match crate::repositories::guild_repo::upsert(pool, &guild_id_str).await {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!("Failed to upsert guild config for join: {:?}", e);
                            return;
                        }
                    }
                }
            };
            state.guild_cache.set(guild_id_str.clone(), config.clone());
            config
        };

        if let Err(e) = crate::services::log_manager::log_member_add(&ctx, &member, guild_id, pool).await {
            tracing::error!("Failed to log member join: {:?}", e);
        }

        if let Err(e) = crate::services::anti_raid::detect_join_raid(&ctx, &member, &guild_config).await {
            tracing::error!("Failed in detect_join_raid: {:?}", e);
        }
    }
}
