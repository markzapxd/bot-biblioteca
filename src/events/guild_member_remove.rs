use serenity::all::{Context, GuildId, Member, User};

pub async fn handle(_ctx: Context, guild_id: GuildId, user: User, _member_data: Option<Member>) {
    tracing::info!("Member left: {} ({}) from guild {}", user.name, user.id, guild_id);
}
