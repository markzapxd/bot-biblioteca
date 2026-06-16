use serenity::all::{Context, Member};

pub async fn handle(_ctx: Context, _old: Option<Member>, new: Member) {
    tracing::info!("Member updated: {} ({})", new.user.name, new.user.id);
}
