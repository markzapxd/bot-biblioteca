use serenity::all::{Command, Context, Ready};
use std::time::Duration;

pub async fn handle(ctx: Context, ready: Ready) {
    tracing::info!("{} is connected!", ready.user.name);

    let _ = Command::set_global_commands(&ctx, vec![]).await;

    let mut cmds = Vec::new();
    crate::commands::register_all(&mut cmds).await;

    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        if let Err(e) = guild_id.set_commands(&ctx, cmds.clone()).await {
            tracing::error!("Failed to register guild commands for {}: {}", guild_id, e);
        } else {
            tracing::info!("Registered guild commands for {}", guild_id);
        }
    }

    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = state.pool.clone();
        if let Err(e) = crate::jobs::voice_sync::sync_voice_states(&ctx, &pool).await {
            tracing::error!("Voice sync failed: {}", e);
        }
        if let Err(e) = crate::repositories::user_repo::recompute_all_voice_times(&pool).await {
            tracing::error!("Failed to recompute all voice times: {}", e);
        }

        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = crate::jobs::voice_sync::update_active_voice_times(&ctx_clone, &pool).await {
                    tracing::error!("Active voice time update failed: {}", e);
                }
            }
        });
    }
}
