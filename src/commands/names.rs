use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::repositories::user_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("names")
            .description("Username history of a user")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let user_id_str = target.to_string();

    let user = user_repo::find_or_create(pool, &user_id_str).await?;

    if user.is_private_mode() {
        let requester = interaction.user.id;
        let is_staff = interaction.member.as_ref().map(|m| permissions::is_admin(m)).unwrap_or(false);
        if requester != target && !is_staff && !permissions::is_owner(requester.get()) {
            let embed = embeds::warning("Private", "This user has privacy mode enabled.");
            let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "names", embed).await;
            let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
            return Ok(());
        }
    }

    let history = user.get_username_history();
    if history.is_empty() {
        let embed = embeds::info("Username History", &format!("<@{}> has no recorded name changes.", target));
        let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "names", embed).await;
        let mut msg = CreateInteractionResponseMessage::new().embed(embed);
        if let Some(file) = attachment {
            msg = msg.add_file(file);
        }
        interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
        return Ok(());
    }

    let mut text = String::new();
    for entry in history.iter().rev().take(20) {
        text.push_str(&format!("**{}** — {}\n", entry.name, entry.date.format("%Y-%m-%d %H:%M")));
    }

    let embed = CreateEmbed::new()
        .title(format!("Username History — {}", target))
        .description(text)
        .colour(Colour::new(0x2B2D31));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "names", embed).await;

    let mut msg = CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

use crate::permissions;
