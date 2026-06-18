use crate::errors::Result;
use chrono::Utc;
use serenity::all::Context;
use sqlx::PgPool;
use std::collections::HashSet;

pub async fn sync_voice_states(ctx: &Context, pool: &PgPool) -> Result<()> {
    let now = Utc::now();
    let orphaned = crate::repositories::voice_session_repo::close_all_active(pool, now).await?;

    let mut affected_users: HashSet<String> = HashSet::new();
    for session in &orphaned {
        affected_users.insert(session.user_id.clone());
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
            affected_users.insert(user_id_str.clone());
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

    for user_id in affected_users {
        if let Err(e) = crate::repositories::user_repo::recompute_voice_time(pool, &user_id).await {
            tracing::error!("Failed to recompute voice time for user {}: {}", user_id, e);
        }
    }

    Ok(())
}

pub async fn update_active_voice_times(ctx: &Context, pool: &PgPool) -> Result<()> {
    let now = Utc::now();

    if let Err(e) = crate::repositories::voice_session_repo::update_active_durations(pool, now).await {
        tracing::error!("Failed to update active durations: {}", e);
    }

    let mut active_users: HashSet<String> = HashSet::new();
    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        if let Some(g) = ctx.cache.guild(guild_id) {
            for (user_id, vs) in g.voice_states.iter() {
                if vs.channel_id.is_some() {
                    let is_bot = g.members.get(user_id)
                        .map(|m| m.user.bot)
                        .unwrap_or(false);
                    if !is_bot {
                        active_users.insert(user_id.to_string());
                    }
                }
            }
        }
    }

    for user_id in active_users {
        if let Err(e) = crate::repositories::user_repo::recompute_voice_time(pool, &user_id).await {
            tracing::error!("Failed to recompute voice time for user {}: {}", user_id, e);
        }
    }

    Ok(())
}
