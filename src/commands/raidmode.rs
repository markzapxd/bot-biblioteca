use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::services::anti_raid;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("raidmode")
            .description("Manually control raid mode")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "on", "Enable raid mode"))
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "off", "Disable raid mode")),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;

    match sub.name.as_str() {
        "on" => {
            anti_raid::trigger_raid(ctx, guild_id, "Manual activation by admin", pool, guild_cache).await?;
            let embed = embeds::warning("Raid Mode Activated", "Lockdown has been applied manually.");
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "off" => {
            anti_raid::disable_raid_mode(ctx, guild_id, pool, guild_cache).await?;
            let embed = embeds::success("Raid Mode Deactivated", "Lockdown has been removed.");
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        _ => {}
    }
    Ok(())
}
