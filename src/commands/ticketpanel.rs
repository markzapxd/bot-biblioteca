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
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_config = _guild_cache.get(&guild_id.to_string())
        .ok_or_else(|| crate::errors::BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    ticket_manager::send_ticket_panel(ctx, interaction.channel_id).await?;
    Ok(())
}
