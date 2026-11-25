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
const AUTO_LOCKDOWN: bool = true;
const LOCKDOWN_DURATION: Duration = Duration::from_secs(300);
const NEW_ACCOUNT_DAYS: i64 = 7;

struct JoinEvent {
    _user_id: u64,
    guild_id: u64,
    timestamp: Instant,
}

struct RaidState {
    active: bool,
    generation: u64,
    triggered_at: Instant,
    auto_disable_at: Instant,
}

static NEXT_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static JOIN_TRACKER: Mutex<Vec<JoinEvent>> = Mutex::new(Vec::new());
static MESSAGE_TRACKER: LazyLock<Mutex<HashMap<(u64, u64), Vec<Instant>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static RAID_STATUS: LazyLock<Mutex<HashMap<u64, RaidState>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn detect_join_raid(ctx: &Context, member: &Member, guild_config: &Guild) -> Result<()> {
    if !guild_config.is_module_enabled("antiraid") {
        return Ok(());
    }

    let now = Instant::now();
    let user_id = member.user.id.get();
    let guild_id = member.guild_id.get();

    let (should_trigger, recent_joins) = {
        let mut tracker = JOIN_TRACKER.lock().unwrap();
        tracker.retain(|e| now.duration_since(e.timestamp) < MASS_JOIN_INTERVAL);
        tracker.push(JoinEvent { _user_id: user_id, guild_id, timestamp: now });

        let count = tracker.iter().filter(|e| e.guild_id == guild_id).count();
        (count >= MASS_JOIN_THRESHOLD, count)
    };

    if should_trigger {
        let reason = format!("Mass join detected: {} joins in {:?}", recent_joins, MASS_JOIN_INTERVAL);
        trigger_raid(ctx, member.guild_id, &reason, guild_config).await?;
    }

    if crate::services::lockdown::Store::new(ctx.data.read().await.get::<crate::state::BotStateKey>().cloned().ok_or_else(|| BotError::Internal("No state".into()))?.pool.clone()).raid_active(guild_id).await? {
        let account_age = chrono::Utc::now() - member.user.created_at().with_timezone(&chrono::Utc);
        if account_age.num_days() < NEW_ACCOUNT_DAYS {
            punish(ctx, member, "New account joining during raid", guild_config).await?;
        }
    }

    Ok(())
}

pub async fn detect_message_violation(ctx: &Context, message: &Message, guild_config: &Guild) -> Result<()> {
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

    let is_spam = {
        let mut tracker = MESSAGE_TRACKER.lock().unwrap();
        let timestamps = tracker.entry((message.guild_id.map(|g| g.get()).unwrap_or(0), user_id)).or_insert_with(Vec::new);
        let now = Instant::now();
        timestamps.retain(|t| now.duration_since(*t) < SPAM_INTERVAL);
        if timestamps.len() >= SPAM_THRESHOLD { timestamps.remove(0); }
        timestamps.push(now);

        timestamps.len() >= SPAM_THRESHOLD
    };

    if is_spam {
        let reason = format!("Spam detected: {} messages in {:?}", SPAM_THRESHOLD, SPAM_INTERVAL);
        if let Some(guild_id) = message.guild_id {
            if let Ok(full_member) = guild_id.member(&ctx.http, message.author.id).await {
                punish_spam(ctx, &full_member, &reason, guild_config).await?;
            }
        }
    }

    Ok(())
}

pub async fn trigger_raid(ctx: &Context, guild: GuildId, reason: &str, guild_config: &Guild) -> Result<()> {
    let generation = NEXT_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let state = ctx.data.read().await.get::<crate::state::BotStateKey>().cloned().ok_or_else(|| BotError::Internal("No bot state".into()))?;
    // Persist ownership and perform effects before any logging await. Failed effects
    // remain pending and the lifecycle reconciler retries them through the same lock.
    crate::services::lockdown::apply_action(ctx, &state.pool, guild, guild_config, crate::services::lockdown::Action::RaidStart(generation)).await?;

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
            let _ = ChannelId::new(channel_id).send_message(&ctx.http, msg).await;
        }
    }


    Ok(())
}

pub async fn disable_raid_mode(ctx: &Context, guild_id: GuildId) -> Result<()> {
    let state = ctx.data.read().await.get::<crate::state::BotStateKey>().cloned().ok_or_else(|| BotError::Internal("No bot state".into()))?;
    let config = crate::repositories::guild_repo::find_by_id(&state.pool, &guild_id.to_string()).await?.ok_or_else(|| BotError::NotFound("Guild config".into()))?;
    crate::services::lockdown::apply_action(ctx, &state.pool, guild_id, &config, crate::services::lockdown::Action::RaidDisable).await?;

    Ok(())
}

pub fn is_raid_active(guild_id: u64) -> bool {
    let status = RAID_STATUS.lock().unwrap();
    status.get(&guild_id).map_or(false, |s| s.active)
}

pub async fn punish_spam(ctx: &Context, member: &Member, reason: &str, guild_config: &Guild) -> Result<()> {
    let member_id = member.user.id.get();
    if crate::permissions::is_immune(member_id, member, guild_config) {
        return Ok(());
    }

    let state = ctx.data.read().await.get::<crate::state::BotStateKey>().cloned()
        .ok_or_else(|| BotError::Internal("No bot state".into()))?;

    let infraction_count = crate::repositories::user_infraction_repo::register_infraction(
        &state.pool,
        &member.guild_id.to_string(),
        &member.user.id.to_string(),
    ).await?;

    let duration_secs = match infraction_count {
        1 => 5 * 60,            // 5 minutes
        2 => 15 * 60,           // 15 minutes
        3 => 60 * 60,           // 1 hour
        4 => 6 * 60 * 60,       // 6 hours
        5 => 24 * 60 * 60,      // 24 hours
        _ => 7 * 24 * 60 * 60,  // 7 days
    };

    let duration_minutes = duration_secs / 60;

    let now = Timestamp::now();
    let future = now.timestamp() + duration_secs as i64;
    let until = Timestamp::from_unix_timestamp(future)
        .map_err(|e| BotError::Internal(e.to_string()))?;

    let edit = EditMember::new().disable_communication_until_datetime(until);
    if let Err(e) = member.guild_id.edit_member(&ctx.http, member.user.id, edit).await {
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
            let _ = ChannelId::new(channel_id).send_message(&ctx.http, msg).await;
        }
    }

    Ok(())
}

pub async fn punish(ctx: &Context, member: &Member, reason: &str, guild_config: &Guild) -> Result<()> {
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
                    member.guild_id.edit_member(&ctx.http, member.user.id, edit).await?;
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

    let embed = crate::embeds::warning("Punishment Applied", &format!("User: {}\nAction: {}\nReason: {}", member.user.name, DEFAULT_ACTION, reason));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "raidmode", embed).await;
    if let Some(log_channel_id) = &guild_config.log_channel_id {
        if let Ok(channel_id) = log_channel_id.parse::<u64>() {
            let mut msg = CreateMessage::new().embed(embed);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            let _ = ChannelId::new(channel_id).send_message(&ctx.http, msg).await;
        }
    }

    Ok(())
}

fn count_emojis(content: &str) -> usize {
    let custom_count = content.matches("<:").count() + content.matches("<a:").count();

    let unicode_count = content.chars().filter(|c| {
        let cp = *c as u32;
        (0x1F600..=0x1F64F).contains(&cp)
            || (0x1F300..=0x1F5FF).contains(&cp)
            || (0x1F680..=0x1F6FF).contains(&cp)
            || (0x2600..=0x26FF).contains(&cp)
            || (0x2700..=0x27BF).contains(&cp)
            || (0xFE00..=0xFE0F).contains(&cp)
            || (0x1F900..=0x1F9FF).contains(&cp)
            || (0x1F1E0..=0x1F1FF).contains(&cp)
    }).count();

    custom_count + unicode_count
}

/// Called by the single lifecycle maintenance loop; no HTTP under these locks.
pub fn prune_trackers(now: Instant) {
    MESSAGE_TRACKER.lock().unwrap().retain(|_, timestamps| {
        timestamps.retain(|t| now.saturating_duration_since(*t) < SPAM_INTERVAL);
        !timestamps.is_empty()
    });
    JOIN_TRACKER.lock().unwrap().retain(|e| now.saturating_duration_since(e.timestamp) < MASS_JOIN_INTERVAL);
    RAID_STATUS.lock().unwrap().retain(|_, s| s.active);
}

pub fn remove_guild(guild: u64) {
    MESSAGE_TRACKER.lock().unwrap().retain(|(g, _), _| *g != guild);
    JOIN_TRACKER.lock().unwrap().retain(|e| e.guild_id != guild);
    RAID_STATUS.lock().unwrap().remove(&guild);
}
