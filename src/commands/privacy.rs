use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::repositories::user_repo;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("privacy")
            .description("Toggle your privacy mode"),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let user_id = interaction.user.id.get();
    let user = user_repo::find_or_create(pool, &user_id.to_string()).await?;

    let is_private = user.is_private_mode();
    let embed = build_privacy_embed(is_private);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "privacy", embed).await;
    let row = build_privacy_row(is_private);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let user_id = component.user.id.get();
    let user = user_repo::find_or_create(pool, &user_id.to_string()).await?;
    let new_status = !user.is_private_mode();
    user_repo::update_privacy(pool, &user_id.to_string(), new_status).await?;

    let embed = build_privacy_embed(new_status);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "privacy", embed).await;
    let row = build_privacy_row(new_status);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

fn build_privacy_embed(is_private: bool) -> CreateEmbed {
    let status = if is_private { "Ativado" } else { "Desativado" };
    theme::info("Modo Privado", &format!("Seu modo privado esta **{}**.", status))
}

fn build_privacy_row(is_private: bool) -> CreateActionRow {
    let btn = CreateButton::new("privacy_toggle")
        .label(if is_private { "Desativar" } else { "Ativar" })
        .style(ButtonStyle::Secondary);
    CreateActionRow::Buttons(vec![btn])
}
