use chrono::{DateTime, Utc};
use serenity::all::*;
use sqlx::PgPool;

use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::repositories::voice_session_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("voicehistory")
            .description("Voice session history of a user")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

fn format_duration(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 && minutes > 0 {
        format!("{}h {}m", hours, minutes)
    } else if hours > 0 {
        format!("{}h", hours)
    } else {
        format!("{}m", minutes)
    }
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    let now = Utc::now();
    if ts.naive_utc().date() == now.naive_utc().date() {
        ts.format("%H:%M").to_string()
    } else {
        ts.format("%d/%m %H:%M").to_string()
    }
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(BotError::Validation("User required".into()))?;
    let user_id_str = target.to_string();

    let (total_sessions, total_duration_ms) = voice_session_repo::get_user_stats(pool, &user_id_str).await?;
    let sessions = voice_session_repo::find_by_user(pool, &user_id_str, 20).await?;

    if sessions.is_empty() {
        let content = format!(
            "Voice History\n\nTotal Sessions: 0\nTotal Voice Time: 0m\nLast Activity: --\n\nNo voice sessions recorded."
        );
        interaction.create_response(ctx, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true)
        )).await?;
        return Ok(());
    }

    let last_activity = sessions
        .first()
        .map(|s| s.left_at.unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now);

    let rows: Vec<(String, String, String)> = sessions
        .iter()
        .map(|s| {
            let end_time = s.left_at.unwrap_or_else(Utc::now);
            let duration_ms = s.duration.unwrap_or_else(|| (end_time - s.joined_at).num_milliseconds());
            (
                s.channel_name.clone(),
                format_duration(duration_ms),
                format_timestamp(end_time),
            )
        })
        .collect();

    let channel_width = rows.iter().map(|r| r.0.len()).max().unwrap_or(0).max("Channel".len());
    let duration_width = rows.iter().map(|r| r.1.len()).max().unwrap_or(0).max("Duration".len());
    let ended_width = rows.iter().map(|r| r.2.len()).max().unwrap_or(0).max("Ended".len());

    let mut table = String::new();
    table.push_str(&format!(
        "{:<cw$}  {:>dw$}  {:<ew$}\n",
        "Channel", "Duration", "Ended",
        cw = channel_width,
        dw = duration_width,
        ew = ended_width,
    ));
    table.push_str(&"-".repeat(channel_width + 2 + duration_width + 2 + ended_width));
    table.push('\n');

    for (channel, duration, ended) in rows {
        table.push_str(&format!(
            "{:<cw$}  {:>dw$}  {:<ew$}\n",
            channel, duration, ended,
            cw = channel_width,
            dw = duration_width,
            ew = ended_width,
        ));
    }

    let content = format!(
        "Voice History\n\nTotal Sessions: {}\nTotal Voice Time: {}\nLast Activity: {}\n\n```\n{}```\nShowing {} sessions",
        total_sessions,
        format_duration(total_duration_ms),
        format_timestamp(last_activity),
        table,
        sessions.len(),
    );

    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(content).ephemeral(true)
    )).await?;

    Ok(())
}
