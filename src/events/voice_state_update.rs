use chrono::Utc;
use serenity::all::{Context, VoiceState};

pub async fn handle(ctx: Context, old: Option<VoiceState>, new: VoiceState) {
    if new.member.as_ref().is_some_and(|m| m.user.bot) { return; }
    if old.as_ref().and_then(|s| s.channel_id) == new.channel_id && old.is_some() { return; }
    let Some(guild) = new.guild_id else { return; };
    let state = ctx.data.read().await.get::<crate::state::BotStateKey>().cloned();
    let Some(state) = state else { return; };
    // Share snapshot/event ordering with cache reconciliation; DB locks remain
    // authoritative across processes and shards.
    let _guard = crate::jobs::voice_sync::LIFECYCLE.lock().await;
    // Read current cache after acquiring the lock, not a potentially stale event.
    let Some(channel) = ctx.cache.guild(guild).map(|g| g.voice_states.get(&new.user_id).and_then(|s| s.channel_id)) else { return; };
    let destination = channel.map(|id| {
        let name = ctx.cache.guild(guild).and_then(|g| g.channels.get(&id).map(|c| c.name.clone())).unwrap_or_else(|| "Unknown".into());
        (id.to_string(), name)
    });
    let previous = crate::repositories::voice_session_repo::find_active_by_user_guild(&state.pool, &new.user_id.to_string(), &guild.to_string()).await.ok().flatten();
    if let Err(e) = crate::repositories::voice_session_repo::transition(&state.pool, &new.user_id.to_string(), &guild.to_string(), None, destination.as_ref().map(|(id,name)| (id.as_str(),name.as_str())), Utc::now()).await {
        tracing::error!("Voice transition failed: {}", e);
        return;
    }
    drop(_guard);
    let name = new.member.as_ref().map(|m| m.user.name.as_str()).unwrap_or("Unknown");
    let face = new.member.as_ref().map(|m| m.user.face()).unwrap_or_default();
    let changed = previous.as_ref().map(|s| s.channel_id.as_str()) != destination.as_ref().map(|d| d.0.as_str());
    if changed {
        if let Some(session) = previous {
            let duration = Utc::now().signed_duration_since(session.joined_at).num_milliseconds().max(0);
            let _ = crate::services::log_manager::log_voice_leave(&ctx, name, &face, &session.channel_name, duration, guild.get(), &state.pool).await;
        }
        if let Some((_, channel_name)) = destination {
            let _ = crate::services::log_manager::log_voice_join(&ctx, name, &face, &channel_name, guild.get(), &state.pool).await;
        }
    }
}
