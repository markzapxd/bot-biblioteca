use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::repositories::{user_repo, voice_session_repo};
use crate::utils::time;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("owner")
            .description("Bot owner commands")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "userinfo", "Full user info (bypass privacy)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "names", "Full name history (bypass privacy)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "voicehistory", "Full voice history (bypass privacy)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "resetuser", "Reset all user data")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "setprivate", "Force set privacy mode")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true))
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Boolean, "status", "Private status").required(true)),
            )
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "botinfo", "Bot statistics"))
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "clearsessions", "Delete all voice sessions")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
            )
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "synccommands", "Re-register slash commands")),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let user_id = interaction.user.id;
    permissions::require_owner(user_id.get())?;

    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;

    match sub.name.as_str() {
        "userinfo" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let user = user_repo::find_or_create(pool, &target.to_string()).await?;
            let embed = CreateEmbed::new()
                .title(format!("Owner UserInfo — {}", target))
                .colour(Colour::new(0x9B59B6))
                .field("User ID", target.to_string(), true)
                .field("Private", user.is_private_mode().to_string(), true)
                .field("Total Voice Time", time::format_duration(user.total_voice_time.unwrap_or(0)), true)
                .field("Premium", user.premium.unwrap_or(false).to_string(), true)
                .field("Last Seen", user.last_seen.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "Never".into()), true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "names" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let user = user_repo::find_or_create(pool, &target.to_string()).await?;
            let history = user.get_username_history();
            let mut text = String::new();
            for entry in history.iter().rev().take(25) {
                text.push_str(&format!("**{}** — {}\n", entry.name, entry.date.format("%Y-%m-%d %H:%M")));
            }
            if text.is_empty() { text = "No history.".into(); }
            let embed = CreateEmbed::new().title(format!("Owner Names — {}", target)).description(text).colour(Colour::new(0x9B59B6));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "voicehistory" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let sessions = voice_session_repo::find_by_user(pool, &target.to_string(), 25).await?;
            let mut text = String::new();
            for s in &sessions {
                text.push_str(&format!("**{}** — {} ({})\n", s.channel_name, s.duration_formatted(), s.joined_at.format("%Y-%m-%d %H:%M")));
            }
            if text.is_empty() { text = "No sessions.".into(); }
            let embed = CreateEmbed::new().title(format!("Owner Voice History — {}", target)).description(text).colour(Colour::new(0x9B59B6));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "resetuser" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            user_repo::reset_user(pool, &target.to_string()).await?;
            let embed = embeds::success("User Reset", &format!("All data for <@{}> has been reset.", target));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "setprivate" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            let status = sub.options.iter().find(|o| o.name == "status").and_then(|o| o.value.as_bool()).unwrap_or(false);
            user_repo::update_privacy(pool, &target.to_string(), status).await?;
            let embed = embeds::success("Privacy Updated", &format!("<@{}> privacy set to {}.", target, status));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "botinfo" => {
            let guilds = ctx.cache.guild_count();
            let users = ctx.cache.user_count();
            let uptime = chrono::Utc::now() - crate::state::BotState { pool: pool.clone(), guild_cache: guild_cache.clone().into(), start_time: chrono::Utc::now() }.start_time;
            let embed = CreateEmbed::new()
                .title("Bot Info")
                .colour(Colour::new(0x9B59B6))
                .field("Guilds", guilds.to_string(), true)
                .field("Cached Users", users.to_string(), true);
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "clearsessions" => {
            let target = sub.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
            user_repo::clear_sessions(pool, &target.to_string()).await?;
            let embed = embeds::success("Sessions Cleared", &format!("All voice sessions for <@{}> deleted.", target));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "synccommands" => {
            let mut cmds = Vec::new();
            crate::commands::register_all(&mut cmds).await;
            let guilds = ctx.cache.guilds();
            for guild_id in guilds {
                let _ = guild_id.set_commands(ctx, &cmds).await;
            }
            let embed = embeds::success("Commands Synced", &format!("Registered {} commands in all guilds.", cmds.len()));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        _ => {}
    }
    Ok(())
}
