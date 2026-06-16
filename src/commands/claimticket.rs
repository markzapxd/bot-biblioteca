use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("claimticket")
            .description("Assumir responsabilidade pelo ticket atual")
            .default_member_permissions(Permissions::MODERATE_MEMBERS),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = serenity::all::CreateEmbed::new()
        .title("Ticket Reivindicado")
        .description(format!("{} assumiu este ticket.", interaction.user.mention()))
        .colour(crate::theme::Theme::SUCCESS);

    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "claimticket", embed).await;

    let mut msg = serenity::all::CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }

    interaction.create_response(ctx, serenity::all::CreateInteractionResponse::Message(msg)).await?;

    Ok(())
}
