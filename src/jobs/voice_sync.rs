use crate::errors::Result;
use chrono::Utc;
use serenity::all::Context;
use sqlx::PgPool;

pub async fn sync_voice_states(ctx: &Context, pool: &PgPool) -> Result<()> {
    let now = Utc::now();
    let orphaned = crate::repositories::voice_session_repo::close_all_active(pool, now).await?;

    for session in &orphaned {
        if let Some(duration) = session.duration {
            if let Err(e) = crate::repositories::user_repo::add_voice_time(pool, &session.user_id, duration).await {
                tracing::error!("Failed to add voice time for user {}: {}", session.user_id, e);
            }
        }
    }

    if let Some(cache) = ctx.cache.as_ref() {
        let guilds = cache.guilds();
        for guild_id in guilds {
            if let Some(guild) = cache.guild(guild_id) {
                for (user_id, voice_state) in &guild.voice_states {
                    if let Some(channel_id) = voice_state.channel_id {
                        if let Some(member) = guild.members.get(user_id) {
                            if !member.user.bot {
                                let channel_name = guild.channels.get(&channel_id)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());

                                if let Err(e) = crate::repositories::voice_session_repo::create(
                                    pool,
                                    &user_id.to_string(),
                                    &guild_id.to_string(),
                                    Some(&guild.name),
                                    &channel_id.to_string(),
                                    &channel_name,
                                    now,
                                ).await {
                                    tracing::error!("Failed to create voice session: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
