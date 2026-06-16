use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::repositories::guild_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("setup")
            .description("Auto-setup bot systems")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "verificacao", "Setup verification system"))
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "logs", "Setup log channel"))
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "setrole", "Set a role in config")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "type", "Role type (member/staff/admin)").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "role", "The role").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "setchannel", "Set a channel in config")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "type", "Channel type (staff/welcome/log)").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "The channel").required(true)),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let guild_id_str = guild_id.to_string();
    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;

    match sub.name.as_str() {
        "verificacao" => {
            interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

            let member_role = guild_id.create_role(ctx, EditRole::new().name("Verificado")).await?;
            let staff_role = guild_id.create_role(ctx, EditRole::new().name("Staff")).await?;

            let verify_channel = guild_id.create_channel(ctx, CreateChannel::new("verificacao").kind(ChannelType::Text)).await?;
            let staff_channel = guild_id.create_channel(ctx, CreateChannel::new("staff").kind(ChannelType::Text)).await?;

            guild_repo::upsert(pool, &guild_id_str).await?;
            guild_repo::update_field(pool, &guild_id_str, "member_role_id", &member_role.id.to_string()).await?;
            guild_repo::update_field(pool, &guild_id_str, "staff_role_id", &staff_role.id.to_string()).await?;
            guild_repo::update_field(pool, &guild_id_str, "welcome_channel_id", &verify_channel.id.to_string()).await?;
            guild_repo::update_field(pool, &guild_id_str, "staff_channel_id", &staff_channel.id.to_string()).await?;

            let embed = embeds::success("Verification Setup Complete", &format!(
                "Member role: <@&{}>\nStaff role: <@&{}>\nVerification channel: <#{}>\nStaff channel: <#{}>",
                member_role.id, staff_role.id, verify_channel.id, staff_channel.id,
            ));
            interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
        }
        "logs" => {
            interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;
            let log_channel = guild_id.create_channel(ctx, CreateChannel::new("logs").kind(ChannelType::Text)).await?;
            guild_repo::upsert(pool, &guild_id_str).await?;
            guild_repo::update_field(pool, &guild_id_str, "log_channel_id", &log_channel.id.to_string()).await?;
            let embed = embeds::success("Logs Setup Complete", &format!("Log channel: <#{}>", log_channel.id));
            interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
        }
        "setrole" => {
            let role_type = sub.options.iter().find(|o| o.name == "type").and_then(|o| o.value.as_str()).ok_or(crate::errors::BotError::Validation("Type required".into()))?;
            let role_id = sub.options.iter().find(|o| o.name == "role").and_then(|o| o.value.as_role_id()).ok_or(crate::errors::BotError::Validation("Role required".into()))?;
            let field = match role_type {
                "member" => "member_role_id",
                "staff" => "staff_role_id",
                "admin" => "admin_role_id",
                _ => {
                    let embed = embeds::error("Invalid Type", "Valid types: member, staff, admin");
                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
                    return Ok(());
                }
            };
            guild_repo::upsert(pool, &guild_id_str).await?;
            guild_repo::update_field(pool, &guild_id_str, field, &role_id.to_string()).await?;
            let embed = embeds::success("Role Set", &format!("`{}` set to <@&{}>", role_type, role_id));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "setchannel" => {
            let channel_type = sub.options.iter().find(|o| o.name == "type").and_then(|o| o.value.as_str()).ok_or(crate::errors::BotError::Validation("Type required".into()))?;
            let channel_id = sub.options.iter().find(|o| o.name == "channel").and_then(|o| o.value.as_channel_id()).ok_or(crate::errors::BotError::Validation("Channel required".into()))?;
            let field = match channel_type {
                "staff" => "staff_channel_id",
                "welcome" => "welcome_channel_id",
                "log" => "log_channel_id",
                _ => {
                    let embed = embeds::error("Invalid Type", "Valid types: staff, welcome, log");
                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
                    return Ok(());
                }
            };
            guild_repo::upsert(pool, &guild_id_str).await?;
            guild_repo::update_field(pool, &guild_id_str, field, &channel_id.to_string()).await?;
            let embed = embeds::success("Channel Set", &format!("`{}` set to <#{}>", channel_type, channel_id));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        _ => {}
    }
    Ok(())
}
