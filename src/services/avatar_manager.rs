use serenity::all::*;
use tracing::error;

use crate::errors::Result;
use crate::models::AvatarEntry;

pub async fn handle_avatar_history(ctx: &Context, interaction: &ComponentInteraction, user_id: u64, page: usize, pool: &sqlx::PgPool) -> Result<()> {
    let user = match crate::repositories::user_repo::find_by_id(pool, &user_id.to_string()).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Usuário não encontrado no banco de dados.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
        Err(e) => {
            error!("Failed to fetch user for avatar history: {}", e);
            return Err(e);
        }
    };

    let history = user.get_avatar_history();
    let (embed, rows) = build_avatar_page(user_id, &history, page);

    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows)
    )).await?;

    Ok(())
}

pub fn build_avatar_page(user_id: u64, history: &[AvatarEntry], page: usize) -> (CreateEmbed, Vec<CreateActionRow>) {
    let total_pages = if history.is_empty() { 1 } else { history.len() };
    let safe_page = page.min(total_pages.saturating_sub(1));

    let embed = if let Some(entry) = history.get(safe_page) {
        CreateEmbed::new()
            .title("Histórico de Avatares")
            .field("Usuário", format!("<@{}>", user_id), true)
            .field("Página", format!("{} / {}", safe_page + 1, total_pages), true)
            .field("Data", entry.date.format("%d/%m/%Y %H:%M").to_string(), true)
            .image(&entry.url)
            .colour(Colour::new(0x3498DB))
    } else {
        CreateEmbed::new()
            .title("Histórico de Avatares")
            .description("Nenhum avatar no histórico.")
            .colour(Colour::new(0x3498DB))
    };

    let prev_disabled = safe_page == 0;
    let next_disabled = safe_page >= total_pages.saturating_sub(1);

    let prev_btn = CreateButton::new(format!("avatar_{}_{}", user_id, safe_page.saturating_sub(1)))
        .label("◀ Anterior")
        .style(ButtonStyle::Primary)
        .disabled(prev_disabled);
    let next_btn = CreateButton::new(format!("avatar_{}_{}", user_id, safe_page + 1))
        .label("▶ Próximo")
        .style(ButtonStyle::Primary)
        .disabled(next_disabled);
    let back_btn = CreateButton::new(format!("userinfo_back_{}", user_id))
        .label("↩ Ficha")
        .style(ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![prev_btn, next_btn, back_btn]);
    (embed, vec![row])
}
