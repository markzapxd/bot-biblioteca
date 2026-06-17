use serenity::all::{Context, Member};

pub async fn handle(ctx: Context, member: Member) {
    tracing::info!("Member joined: {} ({})", member.user.name, member.user.id);
    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = &state.pool;
        let guild_id = member.guild_id.get();

        
        if let Err(e) = crate::services::log_manager::log_member_add(&ctx, &member, guild_id, pool).await {
            tracing::error!("Failed to log member join: {:?}", e);
        }

        
        if let Ok(Some(guild_config)) = crate::repositories::guild_repo::find_by_id(pool, &guild_id.to_string()).await {
            if let Err(e) = crate::services::anti_raid::detect_join_raid(&ctx, &member, &guild_config).await {
                tracing::error!("Failed in detect_join_raid: {:?}", e);
            }
        }
    }
}
