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
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_config = _guild_cache.get(&guild_id.to_string())
        .ok_or_else(|| crate::errors::BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    interaction.defer(ctx).await?;

    let embed = CreateEmbed::new()
        .title("VERIFICAÇÃO")
        .description("Clique no botão abaixo para iniciar seu formulário de entrada.")
        .colour(Colour::new(0x2B2D31));

    let (embed, attachment) = crate::asset_manager::prepare_embed_large(ctx, "verification", embed).await;

    let button = CreateButton::new("request_access")
        .label("Verificar")
        .style(ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![button]);

    let mut msg = EditInteractionResponse::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.new_attachment(file);
    }
    interaction.edit_response(ctx, msg).await?;
    Ok(())
}
