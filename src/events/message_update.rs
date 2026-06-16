use serenity::all::{Context, Message, MessageUpdateEvent};

pub async fn handle(_ctx: Context, _old: Option<Message>, _new: Option<Message>, event: MessageUpdateEvent) {
    tracing::info!("Message updated in channel {}: {}", event.channel_id, event.id);
}
