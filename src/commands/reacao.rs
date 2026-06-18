use serenity::all::*;
use sqlx::PgPool;

use crate::cache::GuildCache;
use crate::errors::{BotError, Result};

const REACTIONS: &[(&str, &str, &str)] = &[
    ("beijo", "kiss", "deu um beijo em"),
    ("abraco", "hug", "deu um abraco em"),
    ("choro", "cry", "esta chorando com"),
    ("tapa", "slap", "deu um tapa em"),
    ("beijinho", "blowkiss", "mandou um beijo para"),
    ("carinho", "pat", "fez carinho em"),
    ("mordida", "bite", "mordeu"),
    ("danca", "dance", "dancou com"),
    ("sorriso", "smile", "sorriu para"),
    ("vergonha", "blush", "ficou sem graca com"),
    ("piscar", "wink", "piscou para"),
    ("acenar", "wave", "acenou para"),
    ("dormir", "sleep", "dormiu ao lado de"),
    ("palmas", "clap", "aplaudiu"),
    ("comer", "feed", "alimentou"),
    ("cutucar", "poke", "cutucou"),
    ("confuso", "confused", "ficou confuso com"),
    ("olhar", "stare", "encarou"),
    ("afeto", "cuddle", "se aconchegou em"),
    ("chutar", "kick", "chutou"),
    ("soco", "punch", "deu um soco em"),
    ("cabecada", "bonk", "deu uma cabecada em"),
    ("idiota", "baka", "achou que"),
    ("highfive", "highfive", "deu high five em"),
    ("atirar", "shoot", "atirou em"),
];

#[derive(serde::Deserialize)]
struct NekosResponse {
    results: Vec<NekosResult>,
}

#[derive(serde::Deserialize)]
struct NekosResult {
    url: String,
    anime_name: Option<String>,
}

pub fn register(commands: &mut Vec<CreateCommand>) {
    let mut command = CreateCommand::new("reacao")
        .description("Reacoes com GIFs de anime");

    for (name, _, description) in REACTIONS {
        command = command.add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, *name, *description)
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::User, "usuario", "Usuario alvo da reacao")
                        .required(false),
                ),
        );
    }

    commands.push(command);
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let opt = interaction.data.options.first()
        .ok_or_else(|| BotError::Validation("Subcomando invalido".into()))?;

    let sub_name = opt.name.as_str();

    let target_user = if let CommandDataOptionValue::SubCommand(opts) = &opt.value {
        opts.iter().find(|o| o.name == "usuario").and_then(|o| {
            if let CommandDataOptionValue::User(id) = o.value {
                Some(id)
            } else {
                None
            }
        })
    } else {
        return Err(BotError::Validation("Subcomando invalido".into()));
    };

    let endpoint = REACTIONS
        .iter()
        .find(|(name, _, _)| *name == sub_name)
        .map(|(_, endpoint, _)| *endpoint)
        .ok_or_else(|| BotError::Validation("Reacao nao encontrada".into()))?;

    let action_text = REACTIONS
        .iter()
        .find(|(name, _, _)| *name == sub_name)
        .map(|(_, _, text)| *text)
        .unwrap_or("reagiu");

    let response_text = reqwest::get(format!("https://nekos.best/api/v2/{}", endpoint))
        .await
        .map_err(|e| BotError::Internal(format!("Falha na API nekos.best: {}", e)))?
        .text()
        .await
        .map_err(|e| BotError::Internal(format!("Falha ao ler corpo da resposta da API: {}", e)))?;

    let response: NekosResponse = serde_json::from_str(&response_text)
        .map_err(|e| BotError::Internal(format!("Falha ao ler resposta da API (json): {}. Corpo: {}", e, response_text)))?;

    let result = response.results.into_iter().next()
        .ok_or_else(|| BotError::Internal("API retornou vazio".into()))?;

    let author_mention = format!("<@{}>", interaction.user.id.get());
    let content = if let Some(target_id) = target_user {
        format!("{} {} <@{}>!", author_mention, action_text, target_id.get())
    } else {
        format!("{} {}!", author_mention, action_text)
    };

    let mut embed = CreateEmbed::new()
        .title(sub_name.to_uppercase())
        .image(result.url)
        .colour(Colour::new(0x2B2D31));

    if let Some(anime_name) = &result.anime_name {
        if !anime_name.is_empty() {
            embed = embed.footer(CreateEmbedFooter::new(format!("Fonte: {}", anime_name)));
        }
    }

    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .embed(embed)
    )).await?;

    Ok(())
}
