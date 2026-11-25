use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::services::ticket_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("closeticket")
            .description("Fechar ticket de um usuario")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "Dono do ticket")
                    .required(true),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    // Fresh membership and owner-or-staff policy are enforced by the service.

    let target = interaction.data.options.first()
        .and_then(|o| o.value.as_user_id())
        .ok_or(crate::errors::BotError::Validation("User required".into()))?;

    ticket_manager::handle_close_ticket_command(ctx, interaction, target).await
}
