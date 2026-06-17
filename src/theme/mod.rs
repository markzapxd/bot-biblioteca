use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter, Timestamp};

pub struct Theme;

impl Theme {
    pub const SUCCESS: Colour = Colour::new(0x2B2D31);
    pub const ERROR: Colour = Colour::new(0x2B2D31);
    pub const WARNING: Colour = Colour::new(0x2B2D31);
    pub const INFO: Colour = Colour::new(0x2B2D31);
    pub const MODERATION: Colour = Colour::new(0x2B2D31);
    pub const NEUTRAL: Colour = Colour::new(0x2B2D31);
}

pub fn success(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::SUCCESS)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn error(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::ERROR)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn warning(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::WARNING)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn info(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::INFO)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn moderation(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::MODERATION)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn neutral(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(Theme::NEUTRAL)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn coloured(title: &str, description: &str, colour: Colour) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(colour)
        .footer(footer())
        .timestamp(Timestamp::now())
}

pub fn bare_coloured(title: &str, description: &str, colour: Colour) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(colour)
}

fn footer() -> CreateEmbedFooter {
    CreateEmbedFooter::new("Bibliotecaria")
}

pub fn timestamp() -> Timestamp {
    Timestamp::now()
}
