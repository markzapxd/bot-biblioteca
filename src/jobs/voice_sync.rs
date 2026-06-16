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

        let guilds = ctx.cache.guilds();
        for guild_id in guilds {
            let entries: Vec<(String, String, String, String)> = {
                let guild = ctx.cache.guild(guild_id);
                match guild {
                    Some(g) => {
                        let guild_name = g.name.clone();
                        g.voice_states.iter()
                            .filter_map(|(user_id, vs)| {
                                let channel_id = vs.channel_id?;
                                let member = g.members.get(user_id)?;
                                if member.user.bot { return None; }
                                let channel_name = g.channels.get(&channel_id)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string());
                                Some((user_id.to_string(), channel_id.to_string(), channel_name, guild_name.clone()))
                            })
                            .collect()
                    }
                    None => continue,
                }
            };

            let guild_id_str = guild_id.to_string();
            for (user_id_str, channel_id_str, channel_name, guild_name) in entries {
                if let Err(e) = crate::repositories::voice_session_repo::create(
                    pool,
                    &user_id_str,
                    &guild_id_str,
                    Some(&guild_name),
                    &channel_id_str,
                    &channel_name,
                    now,
                ).await {
                    tracing::error!("Failed to create voice session: {}", e);
                }
            }
        }

    Ok(())
}
