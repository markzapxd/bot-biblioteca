use serenity::all::{Context, Message};

pub async fn handle(_ctx: Context, msg: Message) {
    if msg.author.bot {
        return;
    }
}
