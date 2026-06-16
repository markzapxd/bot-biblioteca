use serenity::all::{Context, CreateEmbed};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing(level: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_new(level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("Tracing initialized");
}

pub async fn send_log(ctx: &Context, channel_id: u64, embed: CreateEmbed) {
    let channel = serenity::all::ChannelId::new(channel_id);
    if let Err(e) = channel
        .send_message(ctx, serenity::all::CreateMessage::new().embed(embed))
        .await
    {
        error!("Failed to send log message: {}", e);
    }
}
