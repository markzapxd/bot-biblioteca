use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("shutdown")
            .description("Desligar o bot")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let embed = crate::theme::info("Shutdown", "Desligando...");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "shutdown", embed).await;

    let mut msg = CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }

    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        std::process::exit(0);
    });

    Ok(())
}
