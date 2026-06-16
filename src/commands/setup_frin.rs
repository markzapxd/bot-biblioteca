use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::repositories::guild_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("setup-frin")
            .description("Configure monitoring channel")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Monitoring channel").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let channel_id = interaction.data.options.iter().find(|o| o.name == "channel").and_then(|o| o.value.as_channel_id()).ok_or(crate::errors::BotError::Validation("Channel required".into()))?;
    let guild_id_str = guild_id.to_string();

    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, "frin_monitor_channel_id", &channel_id.to_string()).await?;

    let embed = embeds::success("Monitoring Configured", &format!("Monitoring channel set to <#{}>.", channel_id));
    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
    Ok(())
}
