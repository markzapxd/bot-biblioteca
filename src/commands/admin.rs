use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("admin")
            .description("Comandos de moderacao")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "ban", "Banir um usuario")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a banir").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Motivo do ban").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "kick", "Expulsar um usuario")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a expulsar").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Motivo da expulsao").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "mute", "Silenciar um usuario")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a silenciar").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "duration", "Duracao em minutos").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Motivo do silencio").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "unmute", "Remover silenciamento de um usuario")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a remover silencio").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "warn", "Advertir um usuario via DM")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a advertir").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "reason", "Motivo da advertencia").required(false)),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_moderator(user_id, member)?;

    let subcommand = interaction.data.options.first()
        .ok_or(BotError::Validation("Subcommand required".into()))?;

    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return Err(BotError::Validation("Invalid subcommand".into())),
    };

    match subcommand.name.as_str() {
        "ban" => handle_ban(ctx, interaction, sub_options).await,
        "kick" => handle_kick(ctx, interaction, sub_options).await,
        "mute" => handle_mute(ctx, interaction, sub_options).await,
        "unmute" => handle_unmute(ctx, interaction, sub_options).await,
        "warn" => handle_warn(ctx, interaction, sub_options).await,
        _ => Ok(()),
    }
}

fn extract_user(options: &[CommandDataOption]) -> Result<UserId> {
    options.iter()
        .find(|o| o.name == "user")
        .and_then(|o| o.value.as_user_id())
        .ok_or(BotError::Validation("User required".into()))
}

fn extract_reason(options: &[CommandDataOption]) -> Option<String> {
    options.iter()
        .find(|o| o.name == "reason")
        .and_then(|o| o.value.as_str().map(|s| s.to_string()))
}

fn extract_duration(options: &[CommandDataOption]) -> i64 {
    options.iter()
        .find(|o| o.name == "duration")
        .and_then(|o| o.value.as_i64())
        .unwrap_or(10)
}

async fn handle_ban(ctx: &Context, interaction: &CommandInteraction, options: &[CommandDataOption]) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let target = extract_user(options)?;
    let reason = extract_reason(options).unwrap_or_else(|| "Nenhum motivo".into());

    let full_reason = format!("Banido por {} — {}", interaction.user.tag(), reason);
    if let Ok(m) = guild_id.member(ctx, target).await {
        m.ban_with_reason(ctx, 0, &full_reason).await?;
    } else {
        guild_id.ban_with_reason(ctx, target, 0, &full_reason).await?;
    }

    let embed = theme::success("Usuario Banido", &format!("<@{}> foi banido.\nMotivo: {}", target.get(), reason));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "admin", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_kick(ctx: &Context, interaction: &CommandInteraction, options: &[CommandDataOption]) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let target = extract_user(options)?;
    let reason = extract_reason(options).unwrap_or_else(|| "Nenhum motivo".into());

    let full_reason = format!("Expulso por {} — {}", interaction.user.tag(), reason);
    let m = guild_id.member(ctx, target).await?;
    m.kick_with_reason(ctx, &full_reason).await?;

    let embed = theme::success("Usuario Expulso", &format!("<@{}> foi expulso.\nMotivo: {}", target.get(), reason));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "admin", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_mute(ctx: &Context, interaction: &CommandInteraction, options: &[CommandDataOption]) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let target = extract_user(options)?;
    let duration = extract_duration(options);
    let reason = extract_reason(options).unwrap_or_else(|| "Nenhum motivo".into());

    let mut m = guild_id.member(ctx, target).await?;
    let until_ts = (chrono::Utc::now() + chrono::Duration::minutes(duration)).timestamp();
    let since_epoch = Timestamp::from_unix_timestamp(until_ts)
        .map_err(|e| BotError::Internal(e.to_string()))?;
    m.disable_communication_until_datetime(&ctx.http, since_epoch).await?;

    let embed = theme::success("Usuario Silenciado", &format!("<@{}> silenciado por {} minutos.\nMotivo: {}", target.get(), duration, reason));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "admin", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_unmute(ctx: &Context, interaction: &CommandInteraction, options: &[CommandDataOption]) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let target = extract_user(options)?;

    let mut m = guild_id.member(ctx, target).await?;
    m.disable_communication_until_datetime(&ctx.http, Timestamp::from_unix_timestamp(0)
        .map_err(|e| BotError::Internal(e.to_string()))?).await?;

    let embed = theme::success("Silencio Removido", &format!("<@{}> nao esta mais silenciado.", target.get()));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "admin", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_warn(ctx: &Context, interaction: &CommandInteraction, options: &[CommandDataOption]) -> Result<()> {
    let target = extract_user(options)?;
    let reason = extract_reason(options).unwrap_or_else(|| "Nenhum motivo".into());

    let warn_embed = CreateEmbed::new()
        .title("Advertencia")
        .description(format!("Voce recebeu uma advertencia da staff.\nMotivo: {}", reason))
        .colour(Colour::new(0x2B2D31));

    if let Ok(user) = target.to_user(ctx).await {
        if let Ok(dm) = user.create_dm_channel(ctx).await {
            let _ = dm.send_message(ctx, CreateMessage::new().embed(warn_embed)).await;
        }
    }

    let embed = theme::success("Advertencia Enviada", &format!("<@{}> foi advertido.\nMotivo: {}", target.get(), reason));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "admin", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
