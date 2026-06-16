use serenity::all::{Context, Presence};

pub async fn handle(_ctx: Context, presence: Presence) {
    tracing::info!("Presence updated for user {}", presence.user.id);
}
