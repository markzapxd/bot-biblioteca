use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("card")
            .description("Sistema de cards personalizados")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "create", "Criar ou salvar um card")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "name", "Nome unico do card").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "title", "Titulo do card").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "description", "Descricao do card").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "image", "URL da imagem ou GIF").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "color", "Cor em hex (ex: #FF5733)").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "footer", "Texto do rodape").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "edit", "Editar um card existente")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "name", "Nome do card").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "title", "Novo titulo").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "description", "Nova descricao").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "image", "Nova URL da imagem").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "color", "Nova cor em hex").required(false))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "footer", "Novo rodape").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "send", "Enviar um card para o canal atual")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "name", "Nome do card").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "Canal de destino (padrao: atual)").required(false)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "preview", "Visualizar um card antes de enviar")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "name", "Nome do card").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "list", "Listar todos os cards salvos"),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Deletar um card")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "name", "Nome do card").required(true)),
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
        "create" => handle_create(ctx, interaction, pool, &guild_id_str, user_id, sub_options).await,
        "edit" => handle_edit(ctx, interaction, pool, &guild_id_str, sub_options).await,
        "send" => handle_send(ctx, interaction, pool, &guild_id_str, sub_options).await,
        "preview" => handle_preview(ctx, interaction, pool, &guild_id_str, sub_options).await,
        "list" => handle_list(ctx, interaction, pool, &guild_id_str).await,
        "delete" => handle_delete(ctx, interaction, pool, &guild_id_str, sub_options).await,
        _ => Ok(()),
    }
}

fn extract_string(options: &[CommandDataOption], name: &str) -> Option<String> {
    options.iter()
        .find(|o| o.name == name)
        .and_then(|o| o.value.as_str().map(|s| s.to_string()))
}

fn extract_required_string(options: &[CommandDataOption], name: &str) -> Result<String> {
    extract_string(options, name)
        .ok_or_else(|| BotError::Validation(format!("{} required", name)))
}

async fn handle_create(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
    user_id: u64,
    options: &[CommandDataOption],
) -> Result<()> {
    let name = extract_required_string(options, "name")?;
    let title = extract_required_string(options, "title")?;
    let description = extract_string(options, "description");
    let image_url = extract_string(options, "image");
    let color = extract_string(options, "color");
    let footer = extract_string(options, "footer");

    if name.len() > 100 {
        return Err(BotError::Validation("Nome do card muito longo (max 100 caracteres)".into()));
    }

    let card = crate::repositories::custom_card_repo::create(
        pool,
        guild_id,
        &name,
        &title,
        description.as_deref(),
        image_url.as_deref(),
        color.as_deref(),
        footer.as_deref(),
        &user_id.to_string(),
    ).await?;

    let embed = crate::embeds::custom_card::preview(&card);
    let msg = CreateInteractionResponseMessage::new()
        .content(format!("Card `{}` salvo com sucesso!", name))
        .embed(embed)
        .ephemeral(true);

    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_edit(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let name = extract_required_string(options, "name")?;
    let title = extract_string(options, "title");
    let description = extract_string(options, "description");
    let image_url = extract_string(options, "image");
    let color = extract_string(options, "color");
    let footer = extract_string(options, "footer");

    let card = crate::repositories::custom_card_repo::update(
        pool,
        guild_id,
        &name,
        title.as_deref(),
        description.as_deref(),
        image_url.as_deref(),
        color.as_deref(),
        footer.as_deref(),
    ).await?;

    match card {
        Some(card) => {
            let embed = crate::embeds::custom_card::preview(&card);
            let msg = CreateInteractionResponseMessage::new()
                .content(format!("Card `{}` atualizado com sucesso!", name))
                .embed(embed)
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
        None => {
            let msg = CreateInteractionResponseMessage::new()
                .content(format!("Card `{}` nao encontrado.", name))
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
    }
    Ok(())
}

async fn handle_send(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let name = extract_required_string(options, "name")?;

    let channel_id = options.iter()
        .find(|o| o.name == "channel")
        .and_then(|o| o.value.as_channel_id());

    let card = crate::repositories::custom_card_repo::find_by_name(pool, guild_id, &name).await?;

    match card {
        Some(card) => {
            let embed = crate::embeds::custom_card::build(&card);

            let target_channel = channel_id.unwrap_or(interaction.channel_id);

            let builder = CreateMessage::new().embed(embed);
            target_channel.send_message(ctx, builder).await?;

            let msg = CreateInteractionResponseMessage::new()
                .content(format!("Card `{}` enviado em <#{}>!", name, target_channel.get()))
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
        None => {
            let msg = CreateInteractionResponseMessage::new()
                .content(format!("Card `{}` nao encontrado.", name))
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
    }
    Ok(())
}

async fn handle_preview(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let name = extract_required_string(options, "name")?;

    let card = crate::repositories::custom_card_repo::find_by_name(pool, guild_id, &name).await?;

    match card {
        Some(card) => {
            let embed = crate::embeds::custom_card::preview(&card);
            let msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
        None => {
            let msg = CreateInteractionResponseMessage::new()
                .content(format!("Card `{}` nao encontrado.", name))
                .ephemeral(true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        }
    }
    Ok(())
}

async fn handle_list(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
) -> Result<()> {
    let cards = crate::repositories::custom_card_repo::list_by_guild(pool, guild_id).await?;
    let embed = crate::embeds::custom_card::list(&cards);
    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn handle_delete(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    guild_id: &str,
    options: &[CommandDataOption],
) -> Result<()> {
    let name = extract_required_string(options, "name")?;

    let deleted = crate::repositories::custom_card_repo::delete(pool, guild_id, &name).await?;

    let content = if deleted {
        format!("Card `{}` deletado com sucesso!", name)
    } else {
        format!("Card `{}` nao encontrado.", name)
    };

    let msg = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
