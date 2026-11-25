use crate::errors::Result;
use chrono::Utc;
use serenity::all::{Context, GuildId};
use sqlx::PgPool;

/// Client-owned background task. Re-entering startup cannot replace or duplicate
/// the worker; dropping the client owner cancels it, including error exits.
#[derive(Default)]
pub struct VoiceWorker {
    task: Option<tokio::task::JoinHandle<()>>,
}

impl VoiceWorker {
    pub fn start(&mut self, work: impl std::future::Future<Output = ()> + Send + 'static) {
        if self.task.is_none() {
            self.task = Some(tokio::spawn(work));
        }
    }
}

impl Drop for VoiceWorker {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub static LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Reconcile only a guild whose GUILD_CREATE/cache-ready snapshot is available.
/// Repeated snapshots are idempotent; unrelated shards are never closed.
pub async fn initialize_guild(ctx: &Context, pool: &PgPool, guild_id: GuildId) -> Result<()> {
    initialize_cached_guild(&ctx.cache, pool, guild_id).await
}

pub async fn initialize_cached_guild(
    cache: &serenity::cache::Cache,
    pool: &PgPool,
    guild_id: GuildId,
) -> Result<()> {
    let _guard = LIFECYCLE.lock().await;
    let snapshot = cache.guild(guild_id).map(|g| {
        let entries: Vec<_> = g
            .voice_states
            .iter()
            .filter_map(|(user, state)| {
                let channel = state.channel_id?;
                // VoiceState.member is supplied by GUILD_CREATE even when the member
                // cache is incomplete (large guilds).
                let member = state.member.as_ref().or_else(|| g.members.get(user));
                if member.is_some_and(|m| m.user.bot) {
                    return None;
                }
                let name = g
                    .channels
                    .get(&channel)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Unknown".into());
                Some((user.to_string(), channel.to_string(), name))
            })
            .collect();
        (g.name.clone(), entries)
    });
    let Some((name, entries)) = snapshot else {
        return Ok(());
    };
    let guild = guild_id.to_string();
    let active = sqlx::query_as::<_, crate::models::VoiceSession>(
        "SELECT * FROM voice_sessions WHERE guild_id=$1 AND active",
    )
    .bind(&guild)
    .fetch_all(pool)
    .await?;
    for session in active {
        if !entries.iter().any(|(user, _, _)| user == &session.user_id) {
            crate::repositories::voice_session_repo::transition(
                pool,
                &session.user_id,
                &guild,
                Some(&name),
                None,
                Utc::now(),
            )
            .await?;
        }
    }
    for (user, channel, channel_name) in entries {
        crate::repositories::voice_session_repo::transition(
            pool,
            &user,
            &guild,
            Some(&name),
            Some((&channel, &channel_name)),
            Utc::now(),
        )
        .await?;
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

#[cfg(test)]
mod worker_tests {
    use super::VoiceWorker;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[tokio::test]
    async fn repeated_startup_owns_one_worker_and_drop_cancels_it() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (started, ready) = tokio::sync::oneshot::channel();
        let (cancelled, stopped) = tokio::sync::oneshot::channel();
        struct CancelOnDrop(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for CancelOnDrop {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        let mut owner = VoiceWorker::default();
        let count = starts.clone();
        owner.start(async move {
            let _cancel = CancelOnDrop(Some(cancelled));
            count.fetch_add(1, Ordering::SeqCst);
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        let original = owner.task.as_ref().unwrap().id();
        for _ in 0..3 {
            let count = starts.clone();
            owner.start(async move {
                count.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(owner.task.as_ref().unwrap().id(), original);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        drop(owner);
        stopped.await.unwrap();
    }
}
