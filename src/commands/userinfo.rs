use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::services::user_info_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("userinfo")
            .description("Tactical user file")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;

    let result = user_info_manager::build_user_info(ctx, target, guild_id, pool, guild_cache).await?;

    if result.restricted {
        let content = result.content.unwrap_or_else(|| "This user has privacy mode enabled.".into());
        interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content).ephemeral(true))).await?;
        return Ok(());
    }

    let mut msg = CreateInteractionResponseMessage::new();
    if let Some(embed) = result.embed {
        msg = msg.embed(embed);
    }
    if let Some(row) = result.action_row {
        msg = msg.components(vec![row]);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
