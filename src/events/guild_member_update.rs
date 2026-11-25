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

                if old_member.user.avatar != member.user.avatar || old_member.avatar != member.avatar {
                    let new_avatar_url = member.face();
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

            if let Some(previous) = old.as_ref() {
                use crate::repositories::user_repo::{append_history, HistoryKind};
                let now = chrono::Utc::now();
                let mut entries = Vec::new();
                if effective_nick(previous) != effective_nick(&member) {
                    entries.push((HistoryKind::Nickname, serde_json::json!({"name":effective_nick(previous), "date":now, "guild_id":member.guild_id.to_string()})));
                }
                if previous.user.name != member.user.name {
                    entries.push((HistoryKind::Username, serde_json::json!({"name":previous.user.name, "date":now})));
                }
                if previous.user.avatar != member.user.avatar {
                    entries.push((HistoryKind::Avatar, serde_json::json!({"url":previous.user.face(), "date":now})));
                }
                if previous.avatar != member.avatar {
                    entries.push((HistoryKind::Avatar, serde_json::json!({"url":previous.face(), "date":now, "guild_id":member.guild_id.to_string()})));
                }
                for (kind, entry) in entries {
                    if let Err(e) = append_history(pool, &user_id, kind, entry).await {
                        tracing::error!("Failed to append history: {}", e);
                    }
                }
            }
        }
    }
}
