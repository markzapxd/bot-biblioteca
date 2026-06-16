use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::repositories::voice_session_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("lastcall")
            .description("Last voice session of a user")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let user_id_str = target.to_string();

    let session = voice_session_repo::find_last_by_user(pool, &user_id_str).await?;
    match session {
        Some(s) => {
            let embed = CreateEmbed::new()
                .title("Last Voice Session")
                .colour(Colour::new(0x3498DB))
                .field("Channel", &s.channel_name, true)
                .field("Duration", s.duration_formatted(), true)
                .field("Joined", s.joined_at.format("%Y-%m-%d %H:%M").to_string(), true)
                .field("Left", s.left_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "Active".into()), true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        None => {
            let embed = embeds::info("No Sessions", &format!("<@{}> has no voice sessions recorded.", target));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
        }
    }
    Ok(())
}
