use chrono::Utc;
use serenity::all::{Context, VoiceState};

async fn close_active_session(
    ctx: &Context,
    pool: &sqlx::PgPool,
    user_id: &str,
    guild_id: &str,
    guild_id_num: u64,
    user_name: &str,
    face: &str,
) {
    if let Ok(Some(session)) = crate::repositories::voice_session_repo::find_active_by_user_guild(pool, user_id, guild_id).await {
        let now = Utc::now();
        let duration = now.signed_duration_since(session.joined_at).num_milliseconds();
        let members = serde_json::json!([]);
        let channel_name = session.channel_name.clone();

        if let Err(e) = crate::repositories::voice_session_repo::close(pool, session.id, now, duration, members).await {
            tracing::error!("Failed to close voice session: {}", e);
        }
        if let Err(e) = crate::repositories::user_repo::recompute_voice_time(pool, user_id).await {
            tracing::error!("Failed to recompute voice time: {}", e);
        }
        let _ = crate::services::log_manager::log_voice_leave(&ctx, user_name, face, &channel_name, duration, guild_id_num, pool).await;
    }
}

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
        let guild_id_num = new.guild_id.map(|g| g.get()).unwrap_or(0);
        let user_name = new.member.as_ref().map(|m| m.user.name.clone()).unwrap_or_else(|| "Unknown".to_string());
        let face = new.member.as_ref().map(|m| m.user.face()).unwrap_or_default();

        let old_channel = old.as_ref().and_then(|o| o.channel_id);
        let new_channel = new.channel_id;

        if old_channel.is_some() && new_channel.is_none() {
            close_active_session(&ctx, pool, &user_id, &guild_id, guild_id_num, &user_name, &face).await;
        } else if new_channel.is_some() && old_channel != new_channel {
            if old_channel.is_some() {
                close_active_session(&ctx, pool, &user_id, &guild_id, guild_id_num, &user_name, &face).await;
            }

            let channel_id = new_channel.unwrap();
            let channel_name = channel_id.name(&ctx.http).await.unwrap_or_else(|_| "Unknown".to_string());
            let now = Utc::now();
            if let Err(e) = crate::repositories::voice_session_repo::create(pool, &user_id, &guild_id, None, &channel_id.to_string(), &channel_name, now).await {
                tracing::error!("Failed to create voice session: {}", e);
            }
            let _ = crate::services::log_manager::log_voice_join(&ctx, &user_name, &face, &channel_name, guild_id_num, pool).await;
        }
    }
}
