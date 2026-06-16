use serenity::all::{Context, Message, MessageUpdateEvent};

pub async fn handle(ctx: Context, old: Option<Message>, new: Option<Message>, event: MessageUpdateEvent) {
    tracing::info!("Message updated in channel {}: {}", event.channel_id, event.id);
    if let Some(gid) = event.guild_id {
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;
            if let (Some(old_msg), Some(new_msg)) = (old, new) {
                let _ = crate::services::log_manager::log_message_edit(
                    &ctx, 
                    &old_msg.content, 
                    &new_msg.content, 
                    &new_msg.author, 
                    gid.get(), 
                    pool
                ).await;
            }
        }
    }
}
