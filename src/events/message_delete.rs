use serenity::all::{ChannelId, Context, GuildId, MessageId};

pub async fn handle(_ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId, _guild_id: Option<GuildId>) {
    tracing::info!("Message deleted in channel {}: {}", channel_id, deleted_message_id);
}
