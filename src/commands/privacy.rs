use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::repositories::user_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("privacy")
            .description("Toggle your privacy mode")
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "on", "Enable privacy mode"))
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "off", "Disable privacy mode")),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let user_id = interaction.user.id.to_string();
    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;

    let enable = match sub.name.as_str() {
        "on" => true,
        "off" => false,
        _ => return Ok(()),
    };

    user_repo::find_or_create(pool, &user_id).await?;
    user_repo::update_privacy(pool, &user_id, enable).await?;

    let status = if enable { "enabled" } else { "disabled" };
    let embed = embeds::success("Privacy Updated", &format!("Privacy mode has been {}.", status));
    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
    Ok(())
}
