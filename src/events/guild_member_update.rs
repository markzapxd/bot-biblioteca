use serenity::all::{Context, Member};

pub async fn handle(ctx: Context, old: Option<Member>, new: Option<Member>) {
    if let Some(member) = new {
        tracing::info!("Member updated: {} ({})", member.user.name, member.user.id);
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;

            let old_roles = old.as_ref().map(|m| m.roles.as_slice()).unwrap_or(&[]);
            let new_roles = member.roles.as_slice();
            if old_roles != new_roles {
                let _ = crate::services::log_manager::log_member_update(&ctx, old_roles, new_roles, &member, member.guild_id.get(), pool).await;
            }

            let user_id = member.user.id.to_string();
            if let Ok(user) = crate::repositories::user_repo::find_or_create(pool, &user_id).await {
                let mut nickname_history = user.get_nickname_history();
                let mut updated = false;

                if let Some(old_member) = old {
                    let old_nick = old_member.nick.as_deref().unwrap_or(&old_member.user.name);
                    let new_nick = member.nick.as_deref().unwrap_or(&member.user.name);
                    if old_nick != new_nick {
                        nickname_history.push(crate::models::NicknameEntry {
                            name: old_nick.to_string(),
                            date: chrono::Utc::now(),
                        });
                        updated = true;
                    }
                }

                if updated {
                    if let Err(e) = crate::repositories::user_repo::update_nickname_history(pool, &user_id, serde_json::to_value(nickname_history).unwrap_or_default()).await {
                        tracing::error!("Failed to update nickname history: {}", e);
                    }
                }
            }
        }
    }
}
