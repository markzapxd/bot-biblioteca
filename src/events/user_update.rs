use serenity::all::{Context, User};

pub async fn handle(ctx: Context, old: Option<User>, new: User) {
    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = &state.pool;
        let user_id = new.id.to_string();

        if let Ok(user) = crate::repositories::user_repo::find_or_create(pool, &user_id).await {
            let mut username_history = user.get_username_history();
            let mut avatar_history = user.get_avatar_history();
            let mut updated = false;

            if let Some(old_user) = old {
                if old_user.name != new.name {
                    username_history.push(crate::models::UsernameEntry {
                        name: old_user.name,
                        date: chrono::Utc::now(),
                    });
                    updated = true;
                }
                if old_user.avatar != new.avatar {
                    if let Some(avatar) = new.avatar {
                        avatar_history.push(crate::models::AvatarEntry {
                            url: avatar.to_string(),
                            date: chrono::Utc::now(),
                        });
                        updated = true;
                    }
                }
            }

            if updated {
                if let Err(e) = crate::repositories::user_repo::update_username_history(pool, &user_id, serde_json::to_value(username_history).unwrap_or_default()).await {
                    tracing::error!("Failed to update username history: {}", e);
                }
                if let Err(e) = crate::repositories::user_repo::update_avatar_history(pool, &user_id, serde_json::to_value(avatar_history).unwrap_or_default()).await {
                    tracing::error!("Failed to update avatar history: {}", e);
                }
            }
        }
    }
}
