use crate::errors::Result;
use sqlx::PgPool;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub guild: u64,
    pub user: u64,
}
impl Key {
    pub fn new(guild: u64, user: u64) -> Self {
        Self { guild, user }
    }
}

pub struct SupportStore {
    pool: PgPool,
}
impl SupportStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn reserve_ticket(&self, key: Key) -> Result<bool> {
        Ok(sqlx::query(
            "INSERT INTO support_tickets(guild_id,user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(key.guild.to_string())
        .bind(key.user.to_string())
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }
    pub async fn release_ticket(&self, key: Key) -> Result<()> {
        sqlx::query(
            "DELETE FROM support_tickets WHERE guild_id=$1 AND user_id=$2 AND channel_id IS NULL",
        )
        .bind(key.guild.to_string())
        .bind(key.user.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn attach_ticket(&self, key: Key, channel: u64) -> Result<()> {
        sqlx::query("UPDATE support_tickets SET channel_id=$3 WHERE guild_id=$1 AND user_id=$2 AND channel_id IS NULL")
            .bind(key.guild.to_string()).bind(key.user.to_string()).bind(channel.to_string()).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn ticket_channel(&self, key: Key) -> Result<Option<u64>> {
        let value: Option<String> = sqlx::query_scalar("SELECT channel_id FROM support_tickets WHERE guild_id=$1 AND user_id=$2 AND channel_id IS NOT NULL")
            .bind(key.guild.to_string()).bind(key.user.to_string()).fetch_optional(&self.pool).await?;
        Ok(value.and_then(|v| v.parse().ok()))
    }
    pub async fn ticket_owner(&self, guild: u64, channel: u64) -> Result<Option<Key>> {
        let value: Option<String> = sqlx::query_scalar(
            "SELECT user_id FROM support_tickets WHERE guild_id=$1 AND channel_id=$2",
        )
        .bind(guild.to_string())
        .bind(channel.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(value
            .and_then(|v| v.parse().ok())
            .map(|user| Key::new(guild, user)))
    }
    // Hold the row lock across deletion: a failed external effect rolls back and preserves ownership.
    pub async fn close_ticket<F, Fut>(
        &self,
        guild: u64,
        channel: u64,
        caller: u64,
        staff: bool,
        delete: F,
    ) -> Result<bool>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut tx = self.pool.begin().await?;
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT user_id FROM support_tickets WHERE guild_id=$1 AND channel_id=$2 FOR UPDATE",
        )
        .bind(guild.to_string())
        .bind(channel.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(owner) = owner else {
            return Ok(false);
        };
        if !staff && owner != caller.to_string() {
            return Ok(false);
        }
        delete().await?;
        sqlx::query("DELETE FROM support_tickets WHERE guild_id=$1 AND user_id=$2")
            .bind(guild.to_string())
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
    pub async fn reserve_request(&self, key: Key) -> Result<bool> {
        Ok(sqlx::query("INSERT INTO access_requests(guild_id,user_id) VALUES($1,$2) ON CONFLICT(guild_id,user_id) DO UPDATE SET channel_id=NULL,message_id=NULL,created_at=NOW() WHERE access_requests.created_at <= NOW() - INTERVAL '24 hours'")
            .bind(key.guild.to_string()).bind(key.user.to_string()).execute(&self.pool).await?.rows_affected() == 1)
    }
    pub async fn release_request(&self, key: Key) -> Result<()> {
        sqlx::query(
            "DELETE FROM access_requests WHERE guild_id=$1 AND user_id=$2 AND message_id IS NULL",
        )
        .bind(key.guild.to_string())
        .bind(key.user.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn attach_request(&self, key: Key, channel: u64, message: u64) -> Result<()> {
        sqlx::query("UPDATE access_requests SET channel_id=$3,message_id=$4 WHERE guild_id=$1 AND user_id=$2 AND message_id IS NULL")
            .bind(key.guild.to_string()).bind(key.user.to_string()).bind(channel.to_string()).bind(message.to_string()).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn decide<F, Fut>(
        &self,
        key: Key,
        channel: u64,
        message: u64,
        effect: F,
    ) -> Result<bool>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut tx = self.pool.begin().await?;
        let found: Option<String> = sqlx::query_scalar("SELECT user_id FROM access_requests WHERE guild_id=$1 AND user_id=$2 AND channel_id=$3 AND message_id=$4 AND created_at > NOW() - INTERVAL '24 hours' FOR UPDATE")
            .bind(key.guild.to_string()).bind(key.user.to_string()).bind(channel.to_string()).bind(message.to_string()).fetch_optional(&mut *tx).await?;
        if found.is_none() {
            return Ok(false);
        }
        effect().await?;
        sqlx::query("DELETE FROM access_requests WHERE guild_id=$1 AND user_id=$2")
            .bind(key.guild.to_string())
            .bind(key.user.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
}

pub async fn store(ctx: &serenity::all::Context) -> Result<SupportStore> {
    let data = ctx.data.read().await;
    let state = data
        .get::<crate::state::BotStateKey>()
        .ok_or_else(|| crate::errors::BotError::Internal("State unavailable".into()))?;
    Ok(SupportStore::new(state.pool.clone()))
}

/// Fresh membership and roles, never the stale component member snapshot.
pub async fn staff(
    ctx: &serenity::all::Context,
    guild_id: serenity::all::GuildId,
    user: serenity::all::UserId,
    config: &crate::models::Guild,
) -> Result<bool> {
    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let member = guild_id.member(&ctx.http, user).await?;
    #[allow(deprecated)]
    let admin = guild
        .member_permissions(&member)
        .contains(serenity::all::Permissions::ADMINISTRATOR);
    Ok(admin || crate::permissions::is_staff(&member, config))
}
