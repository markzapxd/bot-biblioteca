use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::services::ticket_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("ticketpanel")
            .description("Postar painel de tickets neste canal")
            .default_member_permissions(Permissions::MANAGE_GUILD),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    ticket_manager::send_ticket_panel(ctx, interaction.channel_id).await?;
    Ok(())
}
