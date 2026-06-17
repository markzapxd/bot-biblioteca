use serenity::all::*;
use sqlx::PgPool;
use tracing::error;

use crate::errors::Result;
use crate::models::{AvatarEntry, NicknameEntry, UsernameEntry};

pub fn build_names_page(user_id: u64, history: &[UsernameEntry]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = if history.is_empty() {
        CreateEmbed::new()
            .title("Histórico de Nomes")
            .description("Nenhum nome no histórico.")
            .colour(Colour::new(0x2B2D31))
    } else {
        let mut text = String::new();
        for entry in history.iter().rev().take(20) {
            text.push_str(&format!("**{}** — {}\n", entry.name, entry.date.format("%d/%m/%Y %H:%M")));
        }
        CreateEmbed::new()
            .title("Histórico de Nomes")
            .field("Usuário", format!("<@{}>", user_id), true)
            .description(text)
            .colour(Colour::new(0x2B2D31))
    };

    let row = build_navigation_buttons(user_id, "names");
    (embed, vec![row])
}

pub fn build_nicknames_page(user_id: u64, history: &[NicknameEntry]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = if history.is_empty() {
        CreateEmbed::new()
            .title("Histórico de Apelidos")
            .description("Nenhum apelido no histórico.")
            .colour(Colour::new(0x2B2D31))
    } else {
        let mut text = String::new();
        for entry in history.iter().rev().take(20) {
            text.push_str(&format!("**{}** — {}\n", entry.name, entry.date.format("%d/%m/%Y %H:%M")));
        }
        CreateEmbed::new()
            .title("Histórico de Apelidos")
            .field("Usuário", format!("<@{}>", user_id), true)
            .description(text)
            .colour(Colour::new(0x2B2D31))
    };

    let row = build_navigation_buttons(user_id, "nicknames");
    (embed, vec![row])
}

pub fn build_avatars_page(user_id: u64, history: &[AvatarEntry], page: usize) -> (CreateEmbed, Vec<CreateActionRow>) {
    let total_pages = if history.is_empty() { 1 } else { history.len() };
    let safe_page = page.min(total_pages.saturating_sub(1));

    let embed = if let Some(entry) = history.get(safe_page) {
        CreateEmbed::new()
            .title("Histórico de Avatares")
            .field("Usuário", format!("<@{}>", user_id), true)
            .field("Página", format!("{} / {}", safe_page + 1, total_pages), true)
            .field("Data", entry.date.format("%d/%m/%Y %H:%M").to_string(), true)
            .image(&entry.url)
            .colour(Colour::new(0x2B2D31))
    } else {
        CreateEmbed::new()
            .title("Histórico de Avatares")
            .description("Nenhum avatar no histórico.")
            .colour(Colour::new(0x2B2D31))
    };

    let prev_disabled = safe_page == 0;
    let next_disabled = safe_page >= total_pages.saturating_sub(1);

    let prev_btn = CreateButton::new(format!("history_avatar_{}_{}", user_id, safe_page.saturating_sub(1)))
        .label("Anterior")
        .style(ButtonStyle::Secondary)
        .disabled(prev_disabled);
    let next_btn = CreateButton::new(format!("history_avatar_{}_{}", user_id, safe_page + 1))
        .label("Próximo")
        .style(ButtonStyle::Secondary)
        .disabled(next_disabled);

    let nav_row = build_navigation_buttons(user_id, "avatars");
    let avatar_row = CreateActionRow::Buttons(vec![prev_btn, next_btn]);

    (embed, vec![nav_row, avatar_row])
}

fn build_navigation_buttons(user_id: u64, active: &str) -> CreateActionRow {
    let names_btn = CreateButton::new(format!("history_names_{}", user_id))
        .label("Nomes")
        .style(if active == "names" { ButtonStyle::Primary } else { ButtonStyle::Secondary })
        .disabled(active == "names");
    let avatars_btn = CreateButton::new(format!("history_avatars_{}_0", user_id))
        .label("Avatares")
        .style(if active == "avatars" { ButtonStyle::Primary } else { ButtonStyle::Secondary })
        .disabled(active == "avatars");
    let nicknames_btn = CreateButton::new(format!("history_nicknames_{}", user_id))
        .label("Apelidos")
        .style(if active == "nicknames" { ButtonStyle::Primary } else { ButtonStyle::Secondary })
        .disabled(active == "nicknames");

    CreateActionRow::Buttons(vec![names_btn, avatars_btn, nicknames_btn])
}

pub async fn handle_names_button(ctx: &Context, interaction: &ComponentInteraction, user_id: u64, pool: &PgPool) -> Result<()> {
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
            error!("Failed to fetch user for names history: {}", e);
            return Err(e);
        }
    };

    let history = user.get_username_history();
    let (embed, rows) = build_names_page(user_id, &history);

    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows)
    )).await?;

    Ok(())
}

pub async fn handle_nicknames_button(ctx: &Context, interaction: &ComponentInteraction, user_id: u64, pool: &PgPool) -> Result<()> {
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
            error!("Failed to fetch user for nicknames history: {}", e);
            return Err(e);
        }
    };

    let history = user.get_nickname_history();
    let (embed, rows) = build_nicknames_page(user_id, &history);

    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows)
    )).await?;

    Ok(())
}

pub async fn handle_avatars_button(ctx: &Context, interaction: &ComponentInteraction, user_id: u64, page: usize, pool: &PgPool) -> Result<()> {
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
            error!("Failed to fetch user for avatars history: {}", e);
            return Err(e);
        }
    };

    let history = user.get_avatar_history();
    let (embed, rows) = build_avatars_page(user_id, &history, page);

    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows)
    )).await?;

    Ok(())
}
