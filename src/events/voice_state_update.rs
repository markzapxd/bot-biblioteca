use serenity::all::{Context, VoiceState};

pub async fn handle(ctx: Context, old: Option<VoiceState>, new: VoiceState) {
    let is_bot = new.member.as_ref().map(|m| m.user.bot).unwrap_or(false);
    if is_bot {
        return;
    }

    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = &state.pool;
        let user_id = new.user_id.to_string();
        let guild_id = match new.guild_id {
            Some(id) => id.to_string(),
            None => return,
        };

        let old_channel = old.as_ref().and_then(|o| o.channel_id);
        let new_channel = new.channel_id;

        if old_channel.is_some() && new_channel.is_none() {
            if let Ok(Some(session)) = crate::repositories::voice_session_repo::find_active_by_user_guild(pool, &user_id, &guild_id).await {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(session.joined_at).num_milliseconds();
                let members = serde_json::json!([]);
                if let Err(e) = crate::repositories::voice_session_repo::close(pool, session.id, now, duration, members).await {
                    tracing::error!("Failed to close voice session: {}", e);
                }
                if let Err(e) = crate::repositories::user_repo::add_voice_time(pool, &user_id, duration).await {
                    tracing::error!("Failed to add voice time: {}", e);
                }
            }
        } else if new_channel.is_some() && old_channel != new_channel {
            let channel_id = new_channel.unwrap().to_string();
            let channel_name = "Unknown".to_string();
            let now = chrono::Utc::now();
            if let Err(e) = crate::repositories::voice_session_repo::create(pool, &user_id, &guild_id, None, &channel_id, &channel_name, now).await {
                tracing::error!("Failed to create voice session: {}", e);
            }
        }
    }
}
