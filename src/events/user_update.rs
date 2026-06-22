use serenity::all::{Context, CurrentUser};

pub async fn handle(_ctx: Context, old: Option<CurrentUser>, new: CurrentUser) {
    if let Some(old_user) = old {
        if old_user.name != new.name || old_user.avatar != new.avatar {
            tracing::info!("Bot profile updated: {} ({})", new.name, new.id);
        }
    }
}
