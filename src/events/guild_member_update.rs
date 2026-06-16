use serenity::all::{Context, Member};

pub async fn handle(ctx: Context, old: Option<Member>, new: Option<Member>) {
    if let Some(member) = new {
        tracing::info!("Member updated: {} ({})", member.user.name, member.user.id);
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;
            let old_roles = old.as_ref().map(|m| m.roles.as_slice()).unwrap_or(&[]);
            let new_roles = member.roles.as_slice();
            let _ = crate::services::log_manager::log_member_update(&ctx, old_roles, new_roles, &member, member.guild_id.get(), pool).await;
        }
    }
}
