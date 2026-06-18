use serenity::all::*;
use sqlx::PgPool;

use crate::cache::GuildCache;
use crate::errors::{BotError, Result};

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("emoji")
            .description("Cria um emoji no servidor a partir de uma imagem")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "nome", "Nome do emoji (sem espacos)")
                    .required(true)
                    .max_length(32),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Attachment, "imagem", "Imagem do emoji (PNG, GIF ou JPG)")
                    .required(true),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;

    let options = &interaction.data.options;

    let nome = options
        .iter()
        .find(|o| o.name == "nome")
        .and_then(|o| {
            if let CommandDataOptionValue::String(s) = &o.value {
                Some(s.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| BotError::Validation("Nome nao fornecido".into()))?;

    if nome.len() < 2 {
        return Err(BotError::Validation("Nome do emoji precisa ter pelo menos 2 caracteres".into()));
    }
    if !nome.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(BotError::Validation("Nome do emoji so pode conter letras, numeros e underscores".into()));
    }

    let attachment_id = options
        .iter()
        .find(|o| o.name == "imagem")
        .and_then(|o| {
            if let CommandDataOptionValue::Attachment(id) = o.value {
                Some(id)
            } else {
                None
            }
        })
        .ok_or_else(|| BotError::Validation("Imagem nao fornecida".into()))?;

    let attachment = interaction
        .data
        .resolved
        .attachments
        .get(&attachment_id)
        .cloned()
        .ok_or_else(|| BotError::Validation("Imagem nao encontrada".into()))?;

    interaction.defer(ctx).await?;

    let bytes = reqwest::get(attachment.url)
        .await
        .map_err(|e| BotError::Internal(format!("Falha ao baixar imagem: {}", e)))?
        .bytes()
        .await
        .map_err(|e| BotError::Internal(format!("Falha ao ler imagem: {}", e)))?;

    let mime_type = match attachment.filename.split('.').last() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        _ => return Err(BotError::Validation("Formato de imagem invalido. Use PNG, JPG ou GIF.".into())),
    };

    let data_uri = format!("data:{};base64,", mime_type);
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    let image_data = format!("{}{}", data_uri, encoded);

    match guild_id.create_emoji(&ctx.http, &nome, &image_data).await {
        Ok(created) => {
            interaction
                .edit_response(
                    ctx,
                    EditInteractionResponse::new()
                        .content(format!("Emoji <:{}:{}> criado com sucesso!", created.name, created.id)),
                )
                .await?;
        }
        Err(e) => {
            interaction
                .edit_response(
                    ctx,
                    EditInteractionResponse::new()
                        .content(format!("Erro ao criar emoji: {}", e)),
                )
                .await?;
        }
    }

    Ok(())
}
