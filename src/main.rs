use serenity::all::*;
use std::sync::Arc;

mod asset_manager;
mod cache;
mod commands;
mod config;
mod dashboard;
mod dashboard_auth;
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

    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        for guild in guilds {
            if let Some(state) = ctx.data.read().await.get::<BotStateKey>().cloned() {
                if let Err(e) = jobs::voice_sync::initialize_guild(&ctx, &state.pool, guild).await {
                    tracing::error!("Guild voice initialization failed: {}", e);
                }
            }
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        if let Some(state) = ctx.data.read().await.get::<BotStateKey>().cloned() {
            if let Err(e) = jobs::voice_sync::initialize_guild(&ctx, &state.pool, guild.id).await {
                tracing::error!("Guild voice initialization failed: {}", e);
            }
        }
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

    match dashboard_auth::DashboardAuth::from_env() {
        Ok(Some(auth)) => {
            let port = config.port;
            tokio::spawn(async move {
                let tls = auth.tls;
                let app = dashboard::create_router_with_auth(dashboard_state, Some(auth));
                if tls {
                    let _ = rustls::crypto::ring::default_provider().install_default();
                    let cert = std::env::var("DASHBOARD_TLS_CERT").unwrap_or_default();
                    let key = std::env::var("DASHBOARD_TLS_KEY").unwrap_or_default();
                    let tls = match axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await {
                        Ok(tls) => tls,
                        Err(_) => { tracing::error!("Dashboard disabled: invalid TLS configuration"); return; }
                    };
                    if axum_server::bind_rustls(([0, 0, 0, 0], port).into(), tls)
                        .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.is_err() {
                        tracing::error!("Dashboard TLS server stopped");
                    }
                } else {
                    match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
                        Ok(listener) => { if axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.is_err() { tracing::error!("Dashboard server stopped"); } }
                        Err(_) => tracing::error!("Dashboard disabled: cannot bind loopback listener"),
                    }
                }
            });
        }
        Ok(None) => tracing::info!("Dashboard disabled: administrator credentials absent"),
        Err(_) => tracing::error!("Dashboard disabled: invalid authentication configuration"),
    }

    // Exactly one worker belongs to this client lifecycle, never to READY.
    let worker_pool = pool.clone();
    let worker_cache = client.cache.clone();
    let mut worker = jobs::voice_sync::VoiceWorker::default();
    worker.start(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            for guild in worker_cache.guilds() {
                if worker_cache.guild(guild).is_none() { continue; }
                if let Err(e) = jobs::voice_sync::initialize_cached_guild(&worker_cache, &worker_pool, guild).await {
                    tracing::error!("Voice reconciliation failed: {}", e);
                    continue;
                }
                if let Err(e) = sqlx::query("UPDATE voice_sessions SET duration=GREATEST(0,(EXTRACT(EPOCH FROM (NOW()-joined_at))*1000)::bigint), updated_at=NOW() WHERE guild_id=$1 AND active")
                    .bind(guild.to_string()).execute(&worker_pool).await {
                    tracing::error!("Voice duration update failed: {}", e);
                }
                let users = sqlx::query_scalar::<_, String>("SELECT DISTINCT user_id FROM voice_sessions WHERE guild_id=$1 AND active").bind(guild.to_string()).fetch_all(&worker_pool).await;
                if let Ok(users) = users {
                    for user in users {
                        if let Err(e) = repositories::user_repo::recompute_voice_time(&worker_pool, &user).await {
                            tracing::error!("Voice aggregate update failed: {}", e);
                        }
                    }
                }
            }
        }
    });
    let result = client.start().await;
    drop(worker);
    if let Err(e) = result {
        tracing::error!("Client error: {}", e);
        std::process::exit(1);
    }
}
