use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("v")
            .description("Post verification panel")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let embed = CreateEmbed::new()
        .title("🔐 Verification")
        .description("Click the button below to request access to this server.\nYou will need to indicate who invited you.")
        .colour(Colour::new(0x2ECC71));

    let button = CreateButton::new("request_access")
        .label("REQUISITAR ACESSO")
        .style(ButtonStyle::Success)
        .emoji(ReactionType::Unicode("🔑".into()));

    let row = CreateActionRow::Buttons(vec![button]);

    let msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
