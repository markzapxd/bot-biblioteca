use serenity::all::{Command, Context, Ready};

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
        if let Err(e) = crate::jobs::voice_sync::sync_voice_states(&ctx, &state.pool).await {
            tracing::error!("Voice sync failed: {}", e);
        }
    }
}
