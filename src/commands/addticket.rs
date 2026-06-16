use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::services::ticket_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("addticket")
            .description("Adicionar usuario ao ticket atual")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a adicionar")
                    .required(true),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let target = interaction.data.options.first()
        .and_then(|o| o.value.as_user_id())
        .ok_or(crate::errors::BotError::Validation("User required".into()))?;

    ticket_manager::handle_add_user_to_ticket(ctx, interaction, target).await
}
