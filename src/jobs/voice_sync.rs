use crate::errors::Result;
use chrono::Utc;
use serenity::all::{Context, GuildId};
use sqlx::PgPool;

pub static LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Reconcile only a guild whose GUILD_CREATE/cache-ready snapshot is available.
/// Repeated snapshots are idempotent; unrelated shards are never closed.
pub async fn initialize_guild(ctx: &Context, pool: &PgPool, guild_id: GuildId) -> Result<()> {
    initialize_cached_guild(&ctx.cache, pool, guild_id).await
}

pub async fn initialize_cached_guild(cache: &serenity::cache::Cache, pool: &PgPool, guild_id: GuildId) -> Result<()> {
    let _guard = LIFECYCLE.lock().await;
    let snapshot = cache.guild(guild_id).map(|g| {
        let entries: Vec<_> = g.voice_states.iter().filter_map(|(user, state)| {
            let channel = state.channel_id?;
            // VoiceState.member is supplied by GUILD_CREATE even when the member
            // cache is incomplete (large guilds).
            let member = state.member.as_ref().or_else(|| g.members.get(user));
            if member.is_some_and(|m| m.user.bot) { return None; }
            let name = g.channels.get(&channel).map(|c| c.name.clone()).unwrap_or_else(|| "Unknown".into());
            Some((user.to_string(), channel.to_string(), name))
        }).collect();
        (g.name.clone(), entries)
    });
    let Some((name, entries)) = snapshot else { return Ok(()); };
    let guild = guild_id.to_string();
    let active = sqlx::query_as::<_, crate::models::VoiceSession>("SELECT * FROM voice_sessions WHERE guild_id=$1 AND active")
        .bind(&guild).fetch_all(pool).await?;
    for session in active {
        if !entries.iter().any(|(user,_,_)| user == &session.user_id) {
            crate::repositories::voice_session_repo::transition(pool, &session.user_id, &guild, Some(&name), None, Utc::now()).await?;
        }
    }
    for (user, channel, channel_name) in entries {
        crate::repositories::voice_session_repo::transition(pool, &user, &guild, Some(&name), Some((&channel,&channel_name)), Utc::now()).await?;
    }
    Ok(())
}

pub async fn update_active_voice_times(ctx: &Context, pool: &PgPool) -> Result<()> {
    for guild in ctx.cache.guilds() {
        initialize_guild(ctx, pool, guild).await?;
        sqlx::query("UPDATE voice_sessions SET duration=GREATEST(0,(EXTRACT(EPOCH FROM (NOW()-joined_at))*1000)::bigint), updated_at=NOW() WHERE guild_id=$1 AND active")
            .bind(guild.to_string()).execute(pool).await?;
    }
    Ok(())
}
