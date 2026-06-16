use serenity::all::{Context, GuildId, Member, User};

pub async fn handle(ctx: Context, guild_id: GuildId, user: User, member_data: Option<Member>) {
    tracing::info!("Member left: {} ({}) from guild {}", user.name, user.id, guild_id);
    if let Some(member) = member_data {
        if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
            let pool = &state.pool;
            let _ = crate::services::log_manager::log_member_remove(&ctx, &member, guild_id.get(), pool).await;
        }
    }
}
