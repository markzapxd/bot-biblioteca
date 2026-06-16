use crate::cache::GuildCache;
use crate::dto::{ConfigUpdate, CreateRole, GuildInfo, MessageInfo, SendMessage, StatsResponse};
use crate::errors::{BotError, Result};
use crate::models::Guild;
use crate::repositories::guild_repo;
use crate::utils::time::format_duration;
use axum::extract::{Json, Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use serde_json::json;
use serenity::all::{ChannelId, GuildId};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct DashboardState {
    pub pool: PgPool,
    pub http: Arc<serenity::http::Http>,
    pub guild_cache: Arc<GuildCache>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

pub fn create_router(state: DashboardState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/stats", get(get_stats))
        .route("/api/guilds", get(get_guilds))
        .route("/api/config/:guild_id", get(get_config).post(update_config))
        .route("/api/message", post(send_message))
        .route("/api/role", post(create_role))
        .route("/api/messages/:channel_id", get(get_messages))
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn get_stats(State(state): State<DashboardState>) -> Result<Json<StatsResponse>> {
    let guilds = state.http.get_guilds(None, Some(200)).await.map_err(BotError::Discord)?.len();
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await?;
    let uptime_ms = Utc::now().signed_duration_since(state.start_time).num_milliseconds();
    let uptime = format_duration(uptime_ms);

    Ok(Json(StatsResponse {
        guilds,
        ping: 0,
        users: format!("{}", user_count),
        uptime,
    }))
}

async fn get_guilds(State(state): State<DashboardState>) -> Result<Json<Vec<GuildInfo>>> {
    let guilds = state.http.get_guilds(None, Some(200)).await.map_err(BotError::Discord)?;
    let infos: Vec<GuildInfo> = guilds.into_iter().map(|g| GuildInfo {
        id: g.id.to_string(),
        name: g.name,
        icon: g.icon.map(|i| i.to_string()),
    }).collect();
    Ok(Json(infos))
}

async fn get_config(
    State(state): State<DashboardState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Guild>> {
    if let Some(guild) = state.guild_cache.get(&guild_id) {
        return Ok(Json(guild));
    }
    let guild = guild_repo::find_by_id(&state.pool, &guild_id).await?
        .ok_or(BotError::NotFound("Guild not found".into()))?;
    Ok(Json(guild))
}

async fn update_config(
    State(state): State<DashboardState>,
    Path(guild_id): Path<String>,
    Json(body): Json<ConfigUpdate>,
) -> Result<Json<serde_json::Value>> {
    guild_repo::update_field(&state.pool, &guild_id, &body.key, &body.value).await?;
    if let Some(mut guild) = state.guild_cache.get(&guild_id) {
        match body.key.as_str() {
            "prefix" => guild.prefix = Some(body.value.clone()),
            "member_role_id" => guild.member_role_id = Some(body.value.clone()),
            "staff_channel_id" => guild.staff_channel_id = Some(body.value.clone()),
            "welcome_channel_id" => guild.welcome_channel_id = Some(body.value.clone()),
            "log_channel_id" => guild.log_channel_id = Some(body.value.clone()),
            "admin_role_id" => guild.admin_role_id = Some(body.value.clone()),
            "staff_role_id" => guild.staff_role_id = Some(body.value.clone()),
            "ticket_category_id" => guild.ticket_category_id = Some(body.value.clone()),
            "frin_monitor_channel_id" => guild.frin_monitor_channel_id = Some(body.value.clone()),
            "webhook_url" => guild.webhook_url = Some(body.value.clone()),
            "premium" => guild.premium = Some(body.value.parse().unwrap_or(false)),
            "track_mute" => guild.track_mute = Some(body.value.parse().unwrap_or(false)),
            "track_deaf" => guild.track_deaf = Some(body.value.parse().unwrap_or(false)),
            _ => {}
        }
        state.guild_cache.set(guild_id, guild);
    }
    Ok(Json(json!({ "success": true })))
}

async fn send_message(
    State(state): State<DashboardState>,
    Json(body): Json<SendMessage>,
) -> Result<Json<serde_json::Value>> {
    let channel_id = body.channel_id.parse::<u64>()
        .map_err(|_| BotError::Validation("Invalid channel ID".into()))?;
    let map = json!({ "content": body.content });
    state.http.send_message(ChannelId::new(channel_id), vec![], &map).await
        .map_err(BotError::Discord)?;
    Ok(Json(json!({ "success": true })))
}

async fn create_role(
    State(state): State<DashboardState>,
    Json(body): Json<CreateRole>,
) -> Result<Json<serde_json::Value>> {
    let guild_id = body.guild_id.parse::<u64>()
        .map_err(|_| BotError::Validation("Invalid guild ID".into()))?;
    let color = u64::from_str_radix(body.color.trim_start_matches('#'), 16)
        .map_err(|_| BotError::Validation("Invalid color".into()))?;
    let map = json!({ "name": body.name, "color": color });
    let role = state.http.create_role(GuildId::new(guild_id), &map, None).await
        .map_err(BotError::Discord)?;
    Ok(Json(json!({
        "success": true,
        "role_id": role.id.to_string()
    })))
}

async fn get_messages(
    State(state): State<DashboardState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<MessageInfo>>> {
    let channel_id_num = channel_id.parse::<u64>()
        .map_err(|_| BotError::Validation("Invalid channel ID".into()))?;
    let messages = state.http.get_messages(ChannelId::new(channel_id_num), "?limit=50").await
        .map_err(BotError::Discord)?;
    let infos: Vec<MessageInfo> = messages.into_iter().map(|m| MessageInfo {
        author: m.author.name,
        content: m.content,
        timestamp: m.timestamp.unix_timestamp(),
        bot: m.author.bot,
    }).collect();
    Ok(Json(infos))
}
