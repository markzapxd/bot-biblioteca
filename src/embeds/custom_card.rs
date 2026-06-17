use crate::models::CustomCard;
use serenity::all::{Colour, CreateEmbed};

pub fn build(card: &CustomCard) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(&card.title)
        .colour(Colour::new(card.parse_color()));

    if let Some(desc) = &card.description {
        embed = embed.description(desc);
    }

    if let Some(image_url) = &card.image_url {
        embed = embed.image(image_url);
    }

    if let Some(footer) = &card.footer {
        embed = embed.footer(serenity::all::CreateEmbedFooter::new(footer));
    }

    embed
}

pub fn preview(card: &CustomCard) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("[PREVIEW] {}", card.title))
        .colour(Colour::new(card.parse_color()));

    if let Some(desc) = &card.description {
        embed = embed.description(desc);
    }

    if let Some(image_url) = &card.image_url {
        embed = embed.image(image_url);
    }

    if let Some(footer) = &card.footer {
        embed = embed.footer(serenity::all::CreateEmbedFooter::new(footer));
    }

    embed = embed.field(
        "Info",
        format!("Name: `{}`\nColor: `{}`", card.name, card.color.as_deref().unwrap_or("#2B2D31")),
        false,
    );

    embed
}

pub fn list(cards: &[CustomCard]) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Cards Personalizados")
        .colour(Colour::new(0x2B2D31));

    if cards.is_empty() {
        embed = embed.description("Nenhum card salvo neste servidor.");
    } else {
        let mut description = String::new();
        for card in cards {
            description.push_str(&format!("`{}` — {}\n", card.name, card.title));
        }
        embed = embed.description(description);
    }

    embed
}
