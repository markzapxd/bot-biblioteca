use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("admin")
            .description("Moderation commands")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "ban", "Ban a user")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "User to ban").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Reason")),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "kick", "Kick a user")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "User to kick").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Reason")),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "mute", "Mute a user")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "User to mute").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "minutes", "Duration in minutes").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Reason")),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "unmute", "Unmute a user")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "User to unmute").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "warn", "Warn a user")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "User to warn").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Reason").required(true)),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;
    let sub_name = sub.name.as_str();

    match sub_name {
        "ban" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let reason = sub.options.iter().find(|o| o.name == "reason").and_then(|o| o.value.as_str()).unwrap_or("No reason");
            if let Ok(m) = guild_id.member(ctx, target).await {
                let _ = m.ban_with_reason(ctx, 0, reason).await;
            } else {
                let _ = guild_id.ban_with_reason(ctx, target, 0, reason).await;
            }
            let embed = embeds::success("User Banned", &format!("<@{}> banned.\nReason: {}", target, reason));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "kick" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let reason = sub.options.iter().find(|o| o.name == "reason").and_then(|o| o.value.as_str()).unwrap_or("No reason");
            let mut m = guild_id.member(ctx, target).await?;
            m.kick_with_reason(ctx, reason).await?;
            let embed = embeds::success("User Kicked", &format!("<@{}> kicked.\nReason: {}", target, reason));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "mute" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let minutes = sub.options.iter().find(|o| o.name == "minutes").and_then(|o| o.value.as_i64()).unwrap_or(10);
            let reason = sub.options.iter().find(|o| o.name == "reason").and_then(|o| o.value.as_str()).unwrap_or("No reason");
            let mut m = guild_id.member(ctx, target).await?;
            let until = chrono::Utc::now() + chrono::Duration::minutes(minutes);
            m.disable_communication_until_datetime(ctx, until).await?;
            let embed = embeds::success("User Muted", &format!("<@{}> muted for {} minutes.\nReason: {}", target, minutes, reason));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "unmute" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let mut m = guild_id.member(ctx, target).await?;
            m.enable_communication(ctx).await?;
            let embed = embeds::success("User Unmuted", &format!("<@{}> unmuted.", target));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "warn" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let reason = sub.options.iter().find(|o| o.name == "reason").and_then(|o| o.value.as_str()).unwrap_or("No reason");
            if let Ok(user) = target.to_user(ctx).await {
                if let Ok(dm) = user.create_dm_channel(ctx).await {
                    let warn_embed = CreateEmbed::new().title("Warning").description(format!("You have been warned.\nReason: {}", reason)).colour(Colour::new(0xFFFF00));
                    let _ = dm.send_message(ctx, CreateMessage::new().embed(warn_embed)).await;
                }
            }
            let embed = embeds::success("User Warned", &format!("<@{}> warned.\nReason: {}", target, reason));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        _ => {
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Unknown subcommand.").ephemeral(true))).await?;
        }
    }
    Ok(())
}
