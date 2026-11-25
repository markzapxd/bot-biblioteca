use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serenity::all::*;
use tracing::error;

use crate::errors::{BotError, Result};
use crate::models::Guild;

const MASS_JOIN_THRESHOLD: usize = 5;
const MASS_JOIN_INTERVAL: Duration = Duration::from_secs(10);
const SPAM_THRESHOLD: usize = 7;
const SPAM_INTERVAL: Duration = Duration::from_secs(5);
const MASS_MENTION_THRESHOLD: usize = 5;
const MASS_EMOJI_THRESHOLD: usize = 15;
const DEFAULT_ACTION: &str = "kick";
const MUTE_MINUTES: u64 = 30;
const NEW_ACCOUNT_DAYS: i64 = 7;

// Admission fails open for new keys at saturation: never attribute another
// guild's traffic to a member. Existing keys continue to be checked.
const MESSAGE_KEY_CAPACITY: usize = 10_000;
const JOIN_KEY_CAPACITY: usize = 2_000;
const PRUNE_BUDGET: usize = 256;

struct WindowTracker {
    entries: HashMap<(u64, u64), Vec<Instant>>,
    sweep: std::collections::VecDeque<(u64, u64)>,
    capacity: usize,
    threshold: usize,
    window: Duration,
}
impl WindowTracker {
    fn new(capacity: usize, threshold: usize, window: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            sweep: Default::default(),
            capacity,
            threshold,
            window,
        }
    }
    fn record(&mut self, key: (u64, u64), now: Instant) -> bool {
        if !self.entries.contains_key(&key) {
            if self.entries.len() >= self.capacity {
                return false;
            }
            self.sweep.push_back(key);
            self.entries.insert(key, Vec::new());
        }
        let times = self.entries.get_mut(&key).unwrap();
        times.retain(|t| now.saturating_duration_since(*t) < self.window);
        if times.len() == self.threshold {
            times.remove(0);
        }
        times.push(now);
        times.len() == self.threshold
    }
    fn prune(&mut self, now: Instant, budget: usize) -> usize {
        let work = budget.min(self.sweep.len());
        for _ in 0..work {
            let key = self.sweep.pop_front().unwrap();
            let times = self.entries.get_mut(&key).unwrap();
            times.retain(|t| now.saturating_duration_since(*t) < self.window);
            if times.is_empty() {
                self.entries.remove(&key);
            } else {
                self.sweep.push_back(key);
            }
        }
        work
    }
    fn remove_guild(&mut self, guild: u64) {
        self.entries.retain(|(g, _), _| *g != guild);
        self.sweep.retain(|(g, _)| *g != guild);
    }
}
static JOIN_TRACKER: LazyLock<Mutex<WindowTracker>> = LazyLock::new(|| {
    Mutex::new(WindowTracker::new(
        JOIN_KEY_CAPACITY,
        MASS_JOIN_THRESHOLD,
        MASS_JOIN_INTERVAL,
    ))
});
static MESSAGE_TRACKER: LazyLock<Mutex<WindowTracker>> = LazyLock::new(|| {
    Mutex::new(WindowTracker::new(
        MESSAGE_KEY_CAPACITY,
        SPAM_THRESHOLD,
        SPAM_INTERVAL,
    ))
});

pub async fn detect_join_raid(ctx: &Context, member: &Member, guild_config: &Guild) -> Result<()> {
    if !guild_config.is_module_enabled("antiraid") {
        return Ok(());
    }

    let now = Instant::now();
    let guild_id = member.guild_id.get();
    let should_trigger = JOIN_TRACKER.lock().unwrap().record((guild_id, 0), now);
    let recent_joins = MASS_JOIN_THRESHOLD;

    if should_trigger {
        let reason = format!(
            "Mass join detected: {} joins in {:?}",
            recent_joins, MASS_JOIN_INTERVAL
        );
        trigger_raid(ctx, member.guild_id, &reason, guild_config).await?;
    }

    if crate::services::lockdown::Store::new(
        ctx.data
            .read()
            .await
            .get::<crate::state::BotStateKey>()
            .cloned()
            .ok_or_else(|| BotError::Internal("No state".into()))?
            .pool
            .clone(),
    )
    .raid_active(guild_id)
    .await?
    {
        let account_age = chrono::Utc::now() - member.user.created_at().with_timezone(&chrono::Utc);
        if account_age.num_days() < NEW_ACCOUNT_DAYS {
            punish(ctx, member, "New account joining during raid", guild_config).await?;
        }
    }

    Ok(())
}

pub async fn detect_message_violation(
    ctx: &Context,
    message: &Message,
    guild_config: &Guild,
) -> Result<()> {
    if !guild_config.is_module_enabled("antiraid") {
        return Ok(());
    }

    let user_id = message.author.id.get();

    let mention_count = message.mentions.len();
    if mention_count >= MASS_MENTION_THRESHOLD {
        let reason = format!("Mass mention detected: {} mentions", mention_count);
        if let Some(guild_id) = message.guild_id {
            if let Ok(full_member) = guild_id.member(&ctx.http, message.author.id).await {
                punish_spam(ctx, &full_member, &reason, guild_config).await?;
            }
        }
        return Ok(());
    }

    let emoji_count = count_emojis(&message.content);
    if emoji_count >= MASS_EMOJI_THRESHOLD {
        let reason = format!("Mass emoji detected: {} emojis", emoji_count);
        if let Some(guild_id) = message.guild_id {
            if let Ok(full_member) = guild_id.member(&ctx.http, message.author.id).await {
                punish_spam(ctx, &full_member, &reason, guild_config).await?;
            }
        }
        return Ok(());
    }

    let is_spam = message
        .guild_id
        .map(|guild| {
            MESSAGE_TRACKER
                .lock()
                .unwrap()
                .record((guild.get(), user_id), Instant::now())
        })
        .unwrap_or(false);

    if is_spam {
        let reason = format!(
            "Spam detected: {} messages in {:?}",
            SPAM_THRESHOLD, SPAM_INTERVAL
        );
        if let Some(guild_id) = message.guild_id {
            if let Ok(full_member) = guild_id.member(&ctx.http, message.author.id).await {
                punish_spam(ctx, &full_member, &reason, guild_config).await?;
            }
        }
    }

    Ok(())
}

pub async fn trigger_raid(
    ctx: &Context,
    guild: GuildId,
    reason: &str,
    guild_config: &Guild,
) -> Result<()> {
    crate::services::lockdown::serialize(guild.get(), || async {
        let state = ctx
            .data
            .read()
            .await
            .get::<crate::state::BotStateKey>()
            .cloned()
            .ok_or_else(|| BotError::Internal("No bot state".into()))?;
        // Persist ownership and perform effects before any logging await. Failed effects
        // remain pending and the lifecycle reconciler retries them through the same lock.
        crate::services::lockdown::apply_action_locked(
            ctx,
            &state.pool,
            guild,
            guild_config,
            crate::services::lockdown::Action::RaidStart,
        )
        .await?;
        Ok::<(), BotError>(())
    })
    .await?;

    let guild_name = if let Ok(guild_info) = guild.to_partial_guild(&ctx.http).await {
        guild_info.name.clone()
    } else {
        "Unknown".to_string()
    };

    let embed = crate::embeds::raid_detected(&guild_name, reason);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "raidmode", embed).await;
    if let Some(log_channel_id) = &guild_config.log_channel_id {
        if let Ok(channel_id) = log_channel_id.parse::<u64>() {
            let mut msg = CreateMessage::new().embed(embed);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            let _ = ChannelId::new(channel_id)
                .send_message(&ctx.http, msg)
                .await;
        }
    }

    Ok(())
}

pub async fn disable_raid_mode(ctx: &Context, guild_id: GuildId) -> Result<()> {
    crate::services::lockdown::serialize(guild_id.get(), || async {
        let state = ctx
            .data
            .read()
            .await
            .get::<crate::state::BotStateKey>()
            .cloned()
            .ok_or_else(|| BotError::Internal("No bot state".into()))?;
        let config =
            crate::repositories::guild_repo::find_by_id(&state.pool, &guild_id.to_string())
                .await?
                .ok_or_else(|| BotError::NotFound("Guild config".into()))?;
        crate::services::lockdown::apply_action_locked(
            ctx,
            &state.pool,
            guild_id,
            &config,
            crate::services::lockdown::Action::RaidDisable,
        )
        .await?;

        Ok(())
    })
    .await
}

pub async fn punish_spam(
    ctx: &Context,
    member: &Member,
    reason: &str,
    guild_config: &Guild,
) -> Result<()> {
    let member_id = member.user.id.get();
    if crate::permissions::is_immune(member_id, member, guild_config) {
        return Ok(());
    }

    let state = ctx
        .data
        .read()
        .await
        .get::<crate::state::BotStateKey>()
        .cloned()
        .ok_or_else(|| BotError::Internal("No bot state".into()))?;

    let infraction_count = crate::repositories::user_infraction_repo::register_infraction(
        &state.pool,
        &member.guild_id.to_string(),
        &member.user.id.to_string(),
    )
    .await?;

    let duration_secs = match infraction_count {
        1 => 5 * 60,           // 5 minutes
        2 => 15 * 60,          // 15 minutes
        3 => 60 * 60,          // 1 hour
        4 => 6 * 60 * 60,      // 6 hours
        5 => 24 * 60 * 60,     // 24 hours
        _ => 7 * 24 * 60 * 60, // 7 days
    };

    let duration_minutes = duration_secs / 60;

    let now = Timestamp::now();
    let future = now.timestamp() + duration_secs as i64;
    let until =
        Timestamp::from_unix_timestamp(future).map_err(|e| BotError::Internal(e.to_string()))?;

    let edit = EditMember::new().disable_communication_until_datetime(until);
    if let Err(e) = member
        .guild_id
        .edit_member(&ctx.http, member.user.id, edit)
        .await
    {
        error!("Failed to mute member {}: {}", member.user.id, e);
        return Err(BotError::Discord(e));
    }

    let infraction_text = if infraction_count == 1 {
        "1ª infração".to_string()
    } else {
        format!("{}ª infração", infraction_count)
    };

    let embed = crate::embeds::warning(
        "Anti-Spam: Usuário Silenciado",
        &format!(
            "**Usuário:** <@{}>\n**Motivo:** {}\n**Duração do Mute:** {} minutos ({})\n*O contador de infrações expira após 1 semana sem novas infrações.*",
            member.user.id, reason, duration_minutes, infraction_text
        ),
    );
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "raidmode", embed).await;
    if let Some(log_channel_id) = &guild_config.log_channel_id {
        if let Ok(channel_id) = log_channel_id.parse::<u64>() {
            let mut msg = CreateMessage::new().embed(embed);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            let _ = ChannelId::new(channel_id)
                .send_message(&ctx.http, msg)
                .await;
        }
    }

    Ok(())
}

pub async fn punish(
    ctx: &Context,
    member: &Member,
    reason: &str,
    guild_config: &Guild,
) -> Result<()> {
    let member_id = member.user.id.get();
    if crate::permissions::is_immune(member_id, member, guild_config) {
        return Ok(());
    }

    let result = match DEFAULT_ACTION {
        "kick" => member.kick(&ctx.http).await,
        "ban" => member.ban(&ctx.http, 0).await,
        "mute" => {
            let now = Timestamp::now();
            let future = now.timestamp() + (MUTE_MINUTES * 60) as i64;
            match Timestamp::from_unix_timestamp(future) {
                Ok(until) => {
                    let edit = EditMember::new().disable_communication_until_datetime(until);
                    member
                        .guild_id
                        .edit_member(&ctx.http, member.user.id, edit)
                        .await?;
                    Ok(())
                }
                Err(e) => {
                    error!("Invalid timeout timestamp: {}", e);
                    return Err(BotError::Internal(e.to_string()));
                }
            }
        }
        _ => member.kick(&ctx.http).await,
    };

    if let Err(e) = result {
        error!("Failed to punish member {}: {}", member.user.id, e);
        return Err(BotError::Discord(e));
    }

    let embed = crate::embeds::warning(
        "Punishment Applied",
        &format!(
            "User: {}\nAction: {}\nReason: {}",
            member.user.name, DEFAULT_ACTION, reason
        ),
    );
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "raidmode", embed).await;
    if let Some(log_channel_id) = &guild_config.log_channel_id {
        if let Ok(channel_id) = log_channel_id.parse::<u64>() {
            let mut msg = CreateMessage::new().embed(embed);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            let _ = ChannelId::new(channel_id)
                .send_message(&ctx.http, msg)
                .await;
        }
    }

    Ok(())
}

fn count_emojis(content: &str) -> usize {
    let custom_count = content.matches("<:").count() + content.matches("<a:").count();

    let unicode_count = content
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            (0x1F600..=0x1F64F).contains(&cp)
                || (0x1F300..=0x1F5FF).contains(&cp)
                || (0x1F680..=0x1F6FF).contains(&cp)
                || (0x2600..=0x26FF).contains(&cp)
                || (0x2700..=0x27BF).contains(&cp)
                || (0xFE00..=0xFE0F).contains(&cp)
                || (0x1F900..=0x1F9FF).contains(&cp)
                || (0x1F1E0..=0x1F1FF).contains(&cp)
        })
        .count();

    custom_count + unicode_count
}

/// Called by the single lifecycle maintenance loop; no HTTP under these locks.
pub fn prune_trackers(now: Instant) {
    MESSAGE_TRACKER.lock().unwrap().prune(now, PRUNE_BUDGET);
    JOIN_TRACKER.lock().unwrap().prune(now, PRUNE_BUDGET);
    // Raid ownership lives only in the durable lockdown store, never in an
    // evictable tracker. Recovery and generation checks remain there.
}

/// Gateway lifecycle cleanup only; durable lockdown recovery is independent.
pub fn guild_deleted(cache: &crate::cache::GuildCache, guild: u64, unavailable: bool) {
    if unavailable {
        return;
    }
    remove_guild(guild);
    cache.remove(&guild.to_string());
}

pub fn remove_guild(guild: u64) {
    MESSAGE_TRACKER.lock().unwrap().remove_guild(guild);
    JOIN_TRACKER.lock().unwrap().remove_guild(guild);
}

#[cfg(test)]
mod tracker_tests {
    use super::*;
    #[test]
    fn lifecycle_preserves_unavailable_and_removes_only_permanent_guild() {
        let cache = crate::cache::GuildCache::new();
        let now = Instant::now();
        for guild in [91_001, 91_002] {
            let config = serde_json::from_value(serde_json::json!({
                "guild_id": guild.to_string(), "modules": {},
                "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
            }))
            .unwrap();
            cache.set(guild.to_string(), config);
            MESSAGE_TRACKER.lock().unwrap().record((guild, 7), now);
            JOIN_TRACKER.lock().unwrap().record((guild, 0), now);
        }
        guild_deleted(&cache, 91_001, true);
        for guild in [91_001, 91_002] {
            assert!(cache.get(&guild.to_string()).is_some());
            assert!(MESSAGE_TRACKER
                .lock()
                .unwrap()
                .entries
                .contains_key(&(guild, 7)));
            assert!(JOIN_TRACKER
                .lock()
                .unwrap()
                .entries
                .contains_key(&(guild, 0)));
        }
        guild_deleted(&cache, 91_001, false);
        assert!(cache.get("91001").is_none());
        assert!(cache.get("91002").is_some());
        for tracker in [&MESSAGE_TRACKER, &JOIN_TRACKER] {
            let tracker = tracker.lock().unwrap();
            assert!(!tracker.entries.keys().any(|(g, _)| *g == 91_001));
            assert!(!tracker.sweep.iter().any(|(g, _)| *g == 91_001));
            assert!(tracker.entries.keys().any(|(g, _)| *g == 91_002));
        }
        guild_deleted(&cache, 91_002, false);
    }
    #[test]
    fn join_saturation_readmits_after_expiry_and_cleanup() {
        let now = Instant::now();
        let mut tracker = WindowTracker::new(2, MASS_JOIN_THRESHOLD, MASS_JOIN_INTERVAL);
        tracker.record((1, 0), now);
        tracker.record((2, 0), now);
        for _ in 0..MASS_JOIN_THRESHOLD {
            assert!(!tracker.record((3, 0), now));
        }
        assert!(!tracker.entries.contains_key(&(3, 0)));
        for _ in 1..MASS_JOIN_THRESHOLD - 1 {
            assert!(!tracker.record((1, 0), now));
        }
        assert!(tracker.record((1, 0), now));
        tracker.prune(now + MASS_JOIN_INTERVAL, 1);
        assert!(!tracker.record((3, 0), now + MASS_JOIN_INTERVAL));
        assert!(tracker.entries.contains_key(&(3, 0)));
        tracker.remove_guild(2);
        assert!(!tracker.record((4, 0), now + MASS_JOIN_INTERVAL));
        assert!(tracker.entries.contains_key(&(4, 0)));
        assert_eq!(tracker.entries.len(), 2);
    }
    #[test]
    fn message_keys_have_global_capacity() {
        let now = Instant::now();
        let mut tracker = WindowTracker::new(2, SPAM_THRESHOLD, SPAM_INTERVAL);
        tracker.record((1, 1), now);
        tracker.record((2, 1), now);
        assert!(!tracker.record((3, 1), now));
        assert_eq!(tracker.entries.len(), 2);
        for _ in 1..SPAM_THRESHOLD - 1 {
            assert!(!tracker.record((1, 1), now));
        }
        assert!(tracker.record((1, 1), now));
        assert_eq!(tracker.entries[&(2, 1)].len(), 1);
    }
    #[test]
    fn thresholds_isolation_expiry_and_bounded_prune() {
        for (threshold, window) in [
            (SPAM_THRESHOLD, SPAM_INTERVAL),
            (MASS_JOIN_THRESHOLD, MASS_JOIN_INTERVAL),
        ] {
            let now = Instant::now();
            let mut tracker = WindowTracker::new(4, threshold, window);
            for _ in 0..threshold - 1 {
                assert!(!tracker.record((1, 9), now));
            }
            assert!(!tracker.record((2, 9), now));
            assert!(tracker.record((1, 9), now));
            for _ in 0..100 {
                assert!(tracker.record((1, 9), now));
            }
            assert_eq!(tracker.entries[&(1, 9)].len(), threshold);
            assert!(!tracker.record((1, 9), now + window));
            assert_eq!(tracker.prune(now + window, 1), 1);
            assert_eq!(tracker.entries.len(), 2);
            assert_eq!(tracker.prune(now + window, 1), 1);
            assert_eq!(tracker.entries.len(), 1);
            tracker.remove_guild(2);
            assert!(tracker.entries.contains_key(&(1, 9)));
            tracker.remove_guild(1);
            assert!(tracker.entries.is_empty());
            assert!(tracker.sweep.is_empty());
        }
    }
}
