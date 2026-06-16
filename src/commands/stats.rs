use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::repositories::user_repo;
use crate::utils::time;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(CreateCommand::new("stats").description("Voice time ranking"));
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let ranking = user_repo::get_voice_ranking(pool, &guild_id_str, 10).await?;

    if ranking.is_empty() {
        let embed = CreateEmbed::new().title("Voice Stats").description("No voice sessions recorded yet.").colour(Colour::new(0x3498DB));
        interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
        return Ok(());
    }

    let mut text = String::new();
    for (i, (user_id, duration)) in ranking.iter().enumerate() {
        text.push_str(&format!("**{}.** <@{}> — {}\n", i + 1, user_id, time::format_duration(*duration)));
    }

    let embed = CreateEmbed::new()
        .title("🏆 Voice Time Ranking")
        .description(text)
        .colour(Colour::new(0xF1C40F));

    interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
