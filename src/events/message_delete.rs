use serenity::all::{ChannelId, Context, GuildId, MessageId};

pub async fn handle(ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId, guild_id: Option<GuildId>) {
    tracing::info!("Message deleted in channel {}: {}", channel_id, deleted_message_id);
    if let Some(gid) = guild_id {
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;
            
            
            let msg_info = ctx.cache.message(channel_id, deleted_message_id)
                .map(|m| (m.content.clone(), m.author.name.clone()));
                
            if let Some((content, author_name)) = msg_info {
                let channel_name = channel_id.name(&ctx).await.unwrap_or_else(|_| "unknown".to_string());
                let _ = crate::services::log_manager::log_message_delete(
                    &ctx, 
                    &content, 
                    &author_name, 
                    &channel_name, 
                    gid.get(), 
                    pool
                ).await;
            }
        }
    }
}
