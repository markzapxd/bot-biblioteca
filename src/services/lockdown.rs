use crate::errors::{BotError, Result};
use serenity::all::*;

/// Setup only changes channels proven public by effective @everyone visibility.
/// Restricted and inherited-private channels keep every original overwrite unchanged.
pub fn setup_visible_permissions(
    original: &[PermissionOverwrite],
    everyone: RoleId,
    member: RoleId,
    public: bool,
) -> Vec<PermissionOverwrite> {
    if !public {
        return original.to_vec();
    }
    setup_permissions(original, everyone, member)
}
// Never grant a new role access to a channel already private to @everyone.
pub fn setup_permissions(
    original: &[PermissionOverwrite],
    everyone: RoleId,
    member: RoleId,
) -> Vec<PermissionOverwrite> {
    if original.iter().any(|o| {
        o.kind == PermissionOverwriteType::Role(everyone)
            && o.deny.contains(Permissions::VIEW_CHANNEL)
    }) {
        return original.to_vec();
    }
    let mut result = restrict_permissions(original, Permissions::VIEW_CHANNEL, &[everyone]);
    result.push(PermissionOverwrite {
        kind: PermissionOverwriteType::Role(member),
        allow: Permissions::VIEW_CHANNEL,
        deny: Permissions::empty(),
    });
    result
}

pub fn restrict_permissions(
    original: &[PermissionOverwrite],
    bits: Permissions,
    roles: &[RoleId],
) -> Vec<PermissionOverwrite> {
    let mut result = original.to_vec();
    for role in roles {
        if let Some(o) = result
            .iter_mut()
            .find(|o| o.kind == PermissionOverwriteType::Role(*role))
        {
            o.allow.remove(bits);
            o.deny.insert(bits);
        } else {
            result.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(*role),
                allow: Permissions::empty(),
                deny: bits,
            });
        }
    }
    result
}

pub fn restore_permissions(
    snapshot: Option<serde_json::Value>,
) -> Result<Vec<PermissionOverwrite>> {
    serde_json::from_value(snapshot.ok_or_else(|| {
        BotError::Validation("Missing lockdown snapshot; refusing to unlock".into())
    })?)
    .map_err(|_| BotError::Validation("Invalid lockdown snapshot".into()))
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelSnapshot {
    pub id: u64,
    original: Vec<PermissionOverwrite>,
    locked: Vec<PermissionOverwrite>,
    applied: bool,
}
impl ChannelSnapshot {
    pub fn new(
        id: u64,
        original: Vec<PermissionOverwrite>,
        locked: Vec<PermissionOverwrite>,
    ) -> Self {
        Self {
            id,
            original,
            locked,
            applied: false,
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
struct Record {
    manual: bool,
    raid: Option<u64>,
    #[serde(default)]
    expires: Option<i64>,
    #[serde(default)]
    restoring: bool,
    channels: Vec<ChannelSnapshot>,
}
pub enum Action {
    Manual(bool),
    ToggleManual,
    RaidStart,
    RaidEnd(u64),
    RaidDisable,
    Reconcile(i64),
}
static OPERATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<u64, std::sync::Weak<tokio::sync::Mutex<()>>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
pub(crate) fn operation(guild: u64) -> std::sync::Arc<tokio::sync::Mutex<()>> {
    let mut map = OPERATIONS.lock().unwrap();
    map.retain(|_, v| v.strong_count() > 0);
    if let Some(lock) = map.get(&guild).and_then(|v| v.upgrade()) {
        return lock;
    }
    let lock = std::sync::Arc::new(tokio::sync::Mutex::new(()));
    map.insert(guild, std::sync::Arc::downgrade(&lock));
    lock
}
/// Shared production lifecycle boundary: the closure is not invoked until the
/// guild lock is owned, including its very first asynchronous lookup.
pub async fn serialize<T, F, Fut>(guild: u64, work: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let lock = operation(guild);
    let _guard = lock.lock().await;
    work().await
}
pub struct Store {
    pool: sqlx::PgPool,
}
impl Store {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
    pub async fn raid_active(&self, guild: u64) -> Result<bool> {
        let value: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT channels FROM lockdown_snapshots WHERE guild_id=$1")
                .bind(guild.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(value
            .and_then(|v| v.get("raid").cloned())
            .is_some_and(|v| !v.is_null()))
    }
    pub async fn pending_guilds(&self) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT guild_id FROM lockdown_snapshots")
                .fetch_all(&self.pool)
                .await?,
        )
    }
    async fn save(&self, guild: u64, r: &Record) -> Result<()> {
        let value = serde_json::to_value(r).map_err(|e| BotError::Internal(e.to_string()))?;
        sqlx::query("INSERT INTO lockdown_snapshots(guild_id,channels) VALUES($1,$2) ON CONFLICT(guild_id) DO UPDATE SET channels=EXCLUDED.channels").bind(guild.to_string()).bind(value).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn change<F, Fut>(
        &self,
        guild: u64,
        action: Action,
        channels: Vec<ChannelSnapshot>,
        mut effect: F,
    ) -> Result<bool>
    where
        F: FnMut(u64, Vec<PermissionOverwrite>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let lock = operation(guild);
        let _guard = lock.lock().await;
        self.change_locked(guild, action, channels, effect).await
    }
    async fn change_locked<F, Fut>(
        &self,
        guild: u64,
        action: Action,
        channels: Vec<ChannelSnapshot>,
        mut effect: F,
    ) -> Result<bool>
    where
        F: FnMut(u64, Vec<PermissionOverwrite>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let saved: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT channels FROM lockdown_snapshots WHERE guild_id=$1")
                .bind(guild.to_string())
                .fetch_optional(&self.pool)
                .await?;
        let exists = saved.is_some();
        let mut record: Record = match saved {
            Some(v) => serde_json::from_value(v)
                .map_err(|_| BotError::Validation("Invalid snapshot; refusing mutation".into()))?,
            None => Record {
                channels,
                ..Default::default()
            },
        };
        match action {
            Action::Manual(on) => {
                if !on && !exists {
                    return Err(BotError::Validation(
                        "Missing snapshot; refusing unlock".into(),
                    ));
                }
                record.manual = on;
            }
            Action::ToggleManual => {
                if !record.restoring {
                    record.manual = !record.manual;
                }
            }
            Action::RaidStart => {
                // The database sequence survives process restart and snapshot deletion.
                let generation: i64 = sqlx::query_scalar("SELECT nextval('lockdown_generation')")
                    .fetch_one(&self.pool)
                    .await?;
                record.raid = Some(generation as u64);
                record.expires = Some(chrono::Utc::now().timestamp() + 300);
                record.restoring = false;
            }
            Action::RaidDisable => record.raid = None,
            Action::Reconcile(now) => {
                if record.expires.map_or(true, |expiry| expiry <= now) {
                    record.raid = None;
                }
            }
            Action::RaidEnd(generation) => {
                if record.raid != Some(generation) {
                    return Ok(record.manual || record.raid.is_some());
                }
                record.raid = None;
            }
        }
        let active = record.manual || record.raid.is_some();
        record.restoring = !active;
        self.save(guild, &record).await?;
        for i in 0..record.channels.len() {
            if active && record.channels[i].applied {
                continue;
            }
            let ch = &record.channels[i];
            effect(
                ch.id,
                if active {
                    ch.locked.clone()
                } else {
                    ch.original.clone()
                },
            )
            .await?;
            record.channels[i].applied = active;
            self.save(guild, &record).await?;
        }
        if !active {
            sqlx::query("DELETE FROM lockdown_snapshots WHERE guild_id=$1")
                .bind(guild.to_string())
                .execute(&self.pool)
                .await?;
        }
        Ok(active)
    }
}

pub async fn apply_action(
    ctx: &Context,
    pool: &sqlx::PgPool,
    guild: GuildId,
    config: &crate::models::Guild,
    action: Action,
) -> Result<bool> {
    let lock = operation(guild.get());
    let _guard = lock.lock().await;
    apply_action_locked(ctx, pool, guild, config, action).await
}
pub(crate) async fn apply_action_locked(
    ctx: &Context,
    pool: &sqlx::PgPool,
    guild: GuildId,
    config: &crate::models::Guild,
    action: Action,
) -> Result<bool> {
    let channels = guild.channels(&ctx.http).await?;
    let excluded: Vec<_> = [&config.welcome_channel_id, &config.staff_channel_id]
        .into_iter()
        .filter_map(|s| s.as_ref()?.parse::<u64>().ok())
        .map(ChannelId::new)
        .collect();
    let parents: Vec<_> = excluded
        .iter()
        .filter_map(|id| channels.get(id)?.parent_id)
        .collect();
    let mut roles = vec![guild.everyone_role()];
    if let Some(id) = config.member_role_id.as_ref().and_then(|s| s.parse().ok()) {
        roles.push(RoleId::new(id));
    }
    let snapshots = channels
        .values()
        .filter(|ch| {
            !excluded.contains(&ch.id)
                && !parents.contains(&ch.id)
                && matches!(
                    ch.kind,
                    ChannelType::Text | ChannelType::Voice | ChannelType::Category
                )
        })
        .map(|ch| {
            let locked = restrict_permissions(
                &ch.permission_overwrites,
                Permissions::SEND_MESSAGES
                    | Permissions::SEND_MESSAGES_IN_THREADS
                    | Permissions::CREATE_PUBLIC_THREADS
                    | Permissions::CREATE_PRIVATE_THREADS
                    | Permissions::CONNECT,
                &roles,
            );
            ChannelSnapshot::new(ch.id.get(), ch.permission_overwrites.clone(), locked)
        })
        .collect();
    Store::new(pool.clone())
        .change_locked(guild.get(), action, snapshots, |id, p| async move {
            ChannelId::new(id)
                .edit(&ctx.http, EditChannel::new().permissions(p))
                .await?;
            Ok(())
        })
        .await
}
pub async fn apply(
    ctx: &Context,
    pool: &sqlx::PgPool,
    guild: GuildId,
    config: &crate::models::Guild,
    enable: Option<bool>,
) -> Result<bool> {
    apply_action(
        ctx,
        pool,
        guild,
        config,
        match enable {
            Some(on) => Action::Manual(on),
            None => Action::ToggleManual,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_setup_and_lockdown_preserve_boundaries() {
        assert_eq!(
            setup_visible_permissions(&[], RoleId::new(1), RoleId::new(2), false),
            Vec::<PermissionOverwrite>::new()
        );
        let original = vec![
            PermissionOverwrite {
                kind: PermissionOverwriteType::Role(RoleId::new(1)),
                allow: Permissions::SEND_MESSAGES,
                deny: Permissions::VIEW_CHANNEL,
            },
            PermissionOverwrite {
                kind: PermissionOverwriteType::Member(UserId::new(9)),
                allow: Permissions::VIEW_CHANNEL,
                deny: Permissions::ATTACH_FILES,
            },
        ];
        let setup = setup_permissions(&original, RoleId::new(1), RoleId::new(2));
        assert_eq!(setup, original);
        let locked = restrict_permissions(
            &setup,
            Permissions::SEND_MESSAGES,
            &[RoleId::new(1), RoleId::new(2)],
        );
        assert!(locked[0]
            .deny
            .contains(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES));
        assert_eq!(locked[1], original[1]);
        let persisted = serde_json::to_value(&original).unwrap();
        assert_eq!(restore_permissions(Some(persisted)).unwrap(), original);
        assert!(restore_permissions(None).is_err());
    }
}

#[cfg(test)]
mod orchestration_tests {
    use super::*;
    #[tokio::test]
    async fn production_boundary_orders_paused_start_disable_and_inverse() {
        use std::sync::{Arc, Mutex};
        let active = Arc::new(Mutex::new(false));
        let (entered, wait) = tokio::sync::oneshot::channel();
        let (resume, paused) = tokio::sync::oneshot::channel();
        let a = active.clone();
        let start = tokio::spawn(async move {
            serialize(80101, || async move {
                entered.send(()).unwrap();
                paused.await.unwrap();
                *a.lock().unwrap() = true;
            })
            .await;
        });
        wait.await.unwrap();
        let a = active.clone();
        let disable = tokio::spawn(async move {
            serialize(80101, || async move {
                *a.lock().unwrap() = false;
            })
            .await;
        });
        tokio::task::yield_now().await;
        assert!(!disable.is_finished());
        resume.send(()).unwrap();
        start.await.unwrap();
        disable.await.unwrap();
        assert!(!*active.lock().unwrap());
        serialize(80101, || async {
            *active.lock().unwrap() = false;
        })
        .await;
        serialize(80101, || async {
            *active.lock().unwrap() = true;
        })
        .await;
        assert!(*active.lock().unwrap());
    }
}
