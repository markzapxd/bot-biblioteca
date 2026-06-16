use serenity::all::*;
use std::sync::Arc;

mod asset_manager;
mod cache;
mod commands;
mod config;
mod dashboard;
mod database;
mod dto;
mod embeds;
mod errors;
mod events;
mod jobs;
mod models;
mod permissions;
mod repositories;
mod services;
mod state;
mod tests;
mod theme;
mod utils;

use crate::state::{BotState, BotStateKey};

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        events::ready::handle(ctx, ready).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        events::interaction_create::handle(ctx, interaction).await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        events::voice_state_update::handle(ctx, old, new).await;
    }

    async fn user_update(&self, ctx: Context, old: Option<CurrentUser>, new: CurrentUser) {
        events::user_update::handle(ctx, old, new).await;
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        events::guild_member_add::handle(ctx, member).await;
    }

    async fn guild_member_removal(&self, ctx: Context, guild_id: GuildId, user: User, member_data: Option<Member>) {
        events::guild_member_remove::handle(ctx, guild_id, user, member_data).await;
    }

    async fn guild_member_update(&self, ctx: Context, old: Option<Member>, new: Option<Member>, _event: GuildMemberUpdateEvent) {
        events::guild_member_update::handle(ctx, old, new).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        events::message_create::handle(ctx, msg).await;
    }

    async fn message_delete(&self, ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId, guild_id: Option<GuildId>) {
        events::message_delete::handle(ctx, channel_id, deleted_message_id, guild_id).await;
    }

    async fn message_update(&self, ctx: Context, old: Option<Message>, new: Option<Message>, event: MessageUpdateEvent) {
        events::message_update::handle(ctx, old, new, event).await;
    }

    async fn presence_update(&self, ctx: Context, presence: Presence) {
        events::presence_update::handle(ctx, presence).await;
    }
}

#[tokio::main]
async fn main() {
    let config = match config::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    utils::logger::init_tracing(&config.log_level);

    let pool = match database::create_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = database::run_migrations(&pool).await {
        tracing::error!("Migration error: {}", e);
        std::process::exit(1);
    }

    let guild_cache = Arc::new(cache::GuildCache::new());
    match repositories::guild_repo::find_all(&pool).await {
        Ok(guilds) => guild_cache.load_all(guilds),
        Err(e) => {
            tracing::error!("Failed to load guild configs: {}", e);
            std::process::exit(1);
        }
    }

    let asset_manager = Arc::new(asset_manager::AssetManager::new());

    let bot_state = Arc::new(BotState {
        pool: pool.clone(),
        guild_cache: guild_cache.clone(),
        start_time: chrono::Utc::now(),
        asset_manager,
    });

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_PRESENCES;

    let mut client = match Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Client error: {}", e);
            std::process::exit(1);
        }
    };

    {
        let mut data = client.data.write().await;
        data.insert::<BotStateKey>(bot_state.clone());
    }

    let dashboard_state = dashboard::DashboardState {
        pool: pool.clone(),
        http: client.http.clone(),
        guild_cache: guild_cache.clone(),
        start_time: bot_state.start_time,
    };

    let app = dashboard::create_router(dashboard_state);
    let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind dashboard port: {}", e);
            std::process::exit(1);
        }
    };

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Dashboard server error: {}", e);
        }
    });

    if let Err(e) = client.start().await {
        tracing::error!("Client error: {}", e);
        std::process::exit(1);
    }
}
