use crate::{errors::Result, services::caption};
use serenity::all::*;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("caption")
            .description("Adiciona legenda sobre imagem ou GIF animado")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "texto",
                    "Legenda (até 300 caracteres)",
                )
                .required(true)
                .max_length(300),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "arquivo",
                    "Imagem PNG, JPEG ou GIF",
                )
                .required(true),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction) -> Result<()> {
    interaction.defer(ctx).await?;
    let result = async {
        let text = interaction
            .data
            .options
            .iter()
            .find(|o| o.name == "texto")
            .and_then(|o| o.value.as_str())
            .ok_or_else(|| "Texto não fornecido.".to_string())?
            .to_owned();
        let id = interaction
            .data
            .options
            .iter()
            .find(|o| o.name == "arquivo")
            .and_then(|o| {
                if let CommandDataOptionValue::Attachment(id) = o.value {
                    Some(id)
                } else {
                    None
                }
            })
            .ok_or_else(|| "Arquivo não fornecido.".to_string())?;
        let attachment = interaction
            .data
            .resolved
            .attachments
            .get(&id)
            .ok_or_else(|| "Arquivo não encontrado.".to_string())?;
        let permit = caption::acquire()?;
        let limit = (interaction.attachment_size_limit as usize).min(caption::MAX_BYTES);
        caption::process(&attachment.url, attachment.size, text, limit, permit).await
    }
    .await;
    let response = match result {
        Ok(output) => EditInteractionResponse::new()
            .new_attachment(CreateAttachment::bytes(output.bytes, output.filename)),
        Err(message) => EditInteractionResponse::new()
            .content(message)
            .allowed_mentions(CreateAllowedMentions::new()),
    };
    interaction.edit_response(ctx, response).await?;
    Ok(())
}
