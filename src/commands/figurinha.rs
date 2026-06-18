use serenity::all::*;
use sqlx::PgPool;

use crate::cache::GuildCache;
use crate::errors::{BotError, Result};

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("figurinha")
            .description("Cria uma figurinha no servidor a partir de uma imagem")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "nome", "Nome da figurinha")
                    .required(true)
                    .max_length(30),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Attachment, "imagem", "Imagem da figurinha (PNG ou APNG)")
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

    let filename = attachment.filename.clone();
    let ext = filename.split('.').last().unwrap_or("").to_lowercase();
    let accepts_jpg = ext == "jpg" || ext == "jpeg";
    if ext != "png" && ext != "apng" && !accepts_jpg {
        return Err(BotError::Validation("Figurinhas aceitam PNG, APNG ou JPG.".into()));
    }

    if attachment.size > 2_000_000 {
        return Err(BotError::Validation("Imagem muito grande. Maximo 2 MB antes da conversao.".into()));
    }

    interaction.defer(ctx).await?;

    let bytes = reqwest::get(attachment.url)
        .await
        .map_err(|e| BotError::Internal(format!("Falha ao baixar imagem: {}", e)))?
        .bytes()
        .await
        .map_err(|e| BotError::Internal(format!("Falha ao ler imagem: {}", e)))?;

    let sticker_bytes = if accepts_jpg {
        let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
            .map_err(|_| BotError::Validation("JPG invalido ou corrompido.".into()))?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|_| BotError::Internal("Falha ao converter JPG para PNG.".into()))?;
        cursor.into_inner()
    } else {
        if bytes.len() < 8 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(BotError::Validation("O arquivo nao e um PNG valido.".into()));
        }
        bytes.to_vec()
    };

    if sticker_bytes.len() > 320_000 {
        return Err(BotError::Validation("Imagem muito grande apos conversao. Maximo 320 KB.".into()));
    }

    let sticker = CreateSticker::new(&nome, CreateAttachment::bytes(sticker_bytes, "sticker.png".to_string()))
        .description(format!("Figurinha {} criada via bot", nome))
        .tags("😄");

    match guild_id.create_sticker(&ctx.http, sticker).await {
        Ok(created) => {
            interaction
                .edit_response(
                    ctx,
                    EditInteractionResponse::new()
                        .content(format!("Figurinha `{}` criada com sucesso!", created.name)),
                )
                .await?;
        }
        Err(e) => {
            interaction
                .edit_response(
                    ctx,
                    EditInteractionResponse::new()
                        .content(format!("Erro ao criar figurinha: {}", e)),
                )
                .await?;
        }
    }

    Ok(())
}
