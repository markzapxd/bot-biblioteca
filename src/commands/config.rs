use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;
use crate::repositories::guild_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("config")
            .description("Configurar cargos e canais do servidor")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "staff_role", "Definir cargo de staff")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "role", "Cargo de staff").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "member_role", "Definir cargo verificado (membro)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "role", "Cargo verificado").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "admin_role", "Definir cargo de administrador")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "role", "Cargo de administrador").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "staff_channel", "Definir canal de staff")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Canal de staff").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "welcome_channel", "Definir canal de boas-vindas")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Canal de boas-vindas").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "log_channel", "Definir canal de logs")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Canal de logs").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "ticket_category", "Definir categoria de tickets")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "category", "Categoria de tickets").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "show", "Mostrar configuracao atual"),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let guild_config = guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    let subcommand = interaction.data.options.first()
        .ok_or(BotError::Validation("Subcommand required".into()))?;

    let sub_options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return Err(BotError::Validation("Invalid subcommand".into())),
    };

    match subcommand.name.as_str() {
        "staff_role" => handle_staff_role(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "member_role" => handle_member_role(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "admin_role" => handle_admin_role(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "staff_channel" => handle_staff_channel(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "welcome_channel" => handle_welcome_channel(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "log_channel" => handle_log_channel(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "ticket_category" => handle_ticket_category(ctx, interaction, pool, guild_cache, &guild_id_str, sub_options).await,
        "show" => handle_show(ctx, interaction, &guild_config).await,
        _ => Ok(()),
    }
}

fn extract_role(options: &[CommandDataOption]) -> Option<RoleId> {
    options.iter()
        .find(|o| o.name == "role")
        .and_then(|o| o.value.as_role_id())
}

fn extract_channel(options: &[CommandDataOption]) -> Option<ChannelId> {
    options.iter()
        .find(|o| o.name == "channel" || o.name == "category")
        .and_then(|o| o.value.as_channel_id())
}

async fn handle_staff_role(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let role_id = extract_role(options).ok_or(BotError::Validation("Cargo invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "staff_role_id", &role_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.staff_role_id = Some(role_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Cargo de staff definido para <@&{}>", role_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_member_role(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let role_id = extract_role(options).ok_or(BotError::Validation("Cargo invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "member_role_id", &role_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.member_role_id = Some(role_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Cargo verificado definido para <@&{}>", role_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_admin_role(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let role_id = extract_role(options).ok_or(BotError::Validation("Cargo invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "admin_role_id", &role_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.admin_role_id = Some(role_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Cargo de administrador definido para <@&{}>", role_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_staff_channel(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let channel_id = extract_channel(options).ok_or(BotError::Validation("Canal invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "staff_channel_id", &channel_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.staff_channel_id = Some(channel_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Canal de staff definido para <#{}>", channel_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_welcome_channel(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let channel_id = extract_channel(options).ok_or(BotError::Validation("Canal invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "welcome_channel_id", &channel_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.welcome_channel_id = Some(channel_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Canal de boas-vindas definido para <#{}>", channel_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_log_channel(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let channel_id = extract_channel(options).ok_or(BotError::Validation("Canal invalido".into()))?;
    guild_repo::update_field(pool, guild_id, "log_channel_id", &channel_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.log_channel_id = Some(channel_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Canal de logs definido para <#{}>", channel_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_ticket_category(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_cache: &GuildCache,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let channel_id = extract_channel(options).ok_or(BotError::Validation("Categoria invalida".into()))?;
    guild_repo::update_field(pool, guild_id, "ticket_category_id", &channel_id.to_string()).await?;

    let mut config = guild_cache.get(guild_id).ok_or(BotError::NotFound("Guild config not found".into()))?;
    config.ticket_category_id = Some(channel_id.to_string());
    guild_cache.set(guild_id.to_string(), config);

    let embed = CreateEmbed::new()
        .title("Configuracao Atualizada")
        .description(format!("Categoria de tickets definida para <#{}>", channel_id.get()))
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}

async fn handle_show(
    ctx: &Context,
    interaction: &CommandInteraction,
    guild_config: &crate::models::Guild,
) -> Result<()> {
    let mut desc = String::new();

    desc.push_str("**Cargos**\n");
    if let Some(id) = &guild_config.admin_role_id {
        desc.push_str(&format!("Admin: <@&{}>\n", id));
    } else {
        desc.push_str("Admin: Nao configurado\n");
    }
    if let Some(id) = &guild_config.staff_role_id {
        desc.push_str(&format!("Staff: <@&{}>\n", id));
    } else {
        desc.push_str("Staff: Nao configurado\n");
    }
    if let Some(id) = &guild_config.member_role_id {
        desc.push_str(&format!("Verificado: <@&{}>\n", id));
    } else {
        desc.push_str("Verificado: Nao configurado\n");
    }

    desc.push_str("\n**Canais**\n");
    if let Some(id) = &guild_config.staff_channel_id {
        desc.push_str(&format!("Staff: <#{}>\n", id));
    } else {
        desc.push_str("Staff: Nao configurado\n");
    }
    if let Some(id) = &guild_config.welcome_channel_id {
        desc.push_str(&format!("Boas-vindas: <#{}>\n", id));
    } else {
        desc.push_str("Boas-vindas: Nao configurado\n");
    }
    if let Some(id) = &guild_config.log_channel_id {
        desc.push_str(&format!("Logs: <#{}>\n", id));
    } else {
        desc.push_str("Logs: Nao configurado\n");
    }
    if let Some(id) = &guild_config.ticket_category_id {
        desc.push_str(&format!("Tickets: <#{}>\n", id));
    } else {
        desc.push_str("Tickets: Nao configurado\n");
    }

    let embed = CreateEmbed::new()
        .title("Configuracao do Servidor")
        .description(desc)
        .colour(Colour::new(0x2B2D31));
    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true)
    )).await?;
    Ok(())
}
