use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("verify")
            .description("Post verification panel")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = CreateEmbed::new()
        .title("VERIFICAÇÃO")
        .description("Clique no botão abaixo para iniciar seu formulário de entrada.")
        .colour(Colour::new(0x2B2D31));

    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "verification", embed).await;

    let button = CreateButton::new("request_access")
        .label("Verificar")
        .style(ButtonStyle::Success)
        .emoji(ReactionType::Unicode("✅".into()));

    let row = CreateActionRow::Buttons(vec![button]);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
