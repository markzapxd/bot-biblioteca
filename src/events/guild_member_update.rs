use serenity::all::{Context, Member};

fn effective_nick(member: &Member) -> &str {
    member
        .nick
        .as_deref()
        .or(member.user.global_name.as_deref())
        .unwrap_or(&member.user.name)
}

pub async fn handle(ctx: Context, old: Option<Member>, new: Option<Member>) {
    if let Some(member) = new {
        tracing::info!("Member updated: {} ({})", member.user.name, member.user.id);
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;
            let guild_id = member.guild_id.get();
            let user_id = member.user.id.to_string();

            if let Some(old_member) = old.as_ref() {
                let old_roles = old_member.roles.as_slice();
                let new_roles = member.roles.as_slice();
                if old_roles != new_roles {
                    let _ = crate::services::log_manager::log_member_update(
                        &ctx,
                        old_roles,
                        new_roles,
                        &member,
                        guild_id,
                        pool,
                    )
                    .await;
                }

                let old_nick = effective_nick(old_member);
                let new_nick = effective_nick(&member);
                if old_nick != new_nick {
                    let _ = crate::services::log_manager::log_nickname_update(
                        &ctx,
                        &member.user,
                        old_nick,
                        new_nick,
                        guild_id,
                        pool,
                    )
                    .await;
                }

                if old_member.user.name != member.user.name {
                    let _ = crate::services::log_manager::log_name_update(
                        &ctx,
                        member.user.id,
                        &old_member.user.name,
                        &member.user.name,
                        &member.user.face(),
                        guild_id,
                        pool,
                    )
                    .await;
                }

                if old_member.user.avatar != member.user.avatar {
                    let new_avatar_url = member.user.face();
                    let _ = crate::services::log_manager::log_avatar_update(
                        &ctx,
                        member.user.id,
                        &new_avatar_url,
                        guild_id,
                        pool,
                    )
                    .await;
                }
            }

            if let Ok(user) = crate::repositories::user_repo::find_or_create(pool, &user_id).await
            {
                let mut username_history = user.get_username_history();
                let mut avatar_history = user.get_avatar_history();
                let mut nickname_history = user.get_nickname_history();
                let mut updated = false;

                if let Some(old_member) = old.as_ref() {
                    let old_nick = effective_nick(old_member);
                    let new_nick = effective_nick(&member);
                    if old_nick != new_nick {
                        nickname_history.push(crate::models::NicknameEntry {
                            name: old_nick.to_string(),
                            date: chrono::Utc::now(),
                        });
                        updated = true;
                    }

                    if old_member.user.name != member.user.name {
                        username_history.push(crate::models::UsernameEntry {
                            name: old_member.user.name.clone(),
                            date: chrono::Utc::now(),
                        });
                        updated = true;
                    }

                    if old_member.user.avatar != member.user.avatar {
                        avatar_history.push(crate::models::AvatarEntry {
                            url: old_member.user.face(),
                            date: chrono::Utc::now(),
                        });
                        updated = true;
                    }
                }

                if updated {
                    if let Err(e) = crate::repositories::user_repo::update_username_history(
                        pool,
                        &user_id,
                        serde_json::to_value(username_history).unwrap_or_default(),
                    )
                    .await
                    {
                        tracing::error!("Failed to update username history: {}", e);
                    }
                    if let Err(e) = crate::repositories::user_repo::update_avatar_history(
                        pool,
                        &user_id,
                        serde_json::to_value(avatar_history).unwrap_or_default(),
                    )
                    .await
                    {
                        tracing::error!("Failed to update avatar history: {}", e);
                    }
                    if let Err(e) = crate::repositories::user_repo::update_nickname_history(
                        pool,
                        &user_id,
                        serde_json::to_value(nickname_history).unwrap_or_default(),
                    )
                    .await
                    {
                        tracing::error!("Failed to update nickname history: {}", e);
                    }
                }
            }
        }
    }
}
