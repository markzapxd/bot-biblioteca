use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::repositories::voice_session_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("voicehistory")
            .description("Voice session history of a user")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let user_id_str = target.to_string();

    let sessions = voice_session_repo::find_by_user(pool, &user_id_str, 20).await?;

    if sessions.is_empty() {
        let embed = embeds::info("Voice History", &format!("<@{}> has no voice sessions recorded.", target));
        interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
        return Ok(());
    }

    let requester = interaction.user.id;
    let is_staff = interaction.member.as_ref().map(|m| permissions::is_admin(m)).unwrap_or(false);

    let mut text = String::new();
    for s in &sessions {
        text.push_str(&format!(
            "**{}** — {} ({}) — {}\n",
            s.channel_name,
            s.duration_formatted(),
            s.joined_at.format("%Y-%m-%d %H:%M"),
            if s.is_active() { "🟢 Active" } else { "⚫ Ended" },
        ));
    }

    let embed = CreateEmbed::new()
        .title(format!("Voice History — {}", target))
        .description(text)
        .colour(Colour::new(0x3498DB))
        .footer(CreateEmbedFooter::new(format!("{} sessions shown", sessions.len())));

    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
    Ok(())
}
