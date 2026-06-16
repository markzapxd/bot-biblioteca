use serenity::all::{Context, Member};

pub async fn handle(_ctx: Context, member: Member) {
    tracing::info!("Member joined: {} ({})", member.user.name, member.user.id);
}
