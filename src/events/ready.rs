use serenity::all::{Context, Ready};

pub async fn handle(ctx: Context, ready: Ready) {
    tracing::info!("{} is connected!", ready.user.name);
    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        if let Err(e) = crate::jobs::voice_sync::sync_voice_states(&ctx, &state.pool).await {
            tracing::error!("Voice sync failed: {}", e);
        }
    }
}
