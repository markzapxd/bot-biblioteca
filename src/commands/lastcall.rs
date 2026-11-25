use crate::cache::GuildCache;
use crate::embeds;
use crate::errors::Result;
use crate::repositories::voice_session_repo;
use serenity::all::*;
use sqlx::PgPool;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("lastcall")
            .description("Last voice session of a user")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "Target user")
                    .required(true),
            ),
    );
}

pub async fn handle(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
    _guild_cache: &GuildCache,
) -> Result<()> {
    let guild_id = interaction
        .guild_id
        .ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let target = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| o.value.as_user_id())
        .ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let profile = crate::repositories::user_repo::find_by_id(pool, &target.to_string()).await?;
    if !crate::permissions::can_view_profile(
        ctx,
        Some(guild_id),
        interaction.user.id,
        target.get(),
        profile.is_none_or(|u| u.is_private_mode()),
    )
    .await
    {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Este perfil é privado.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }
    let user_id_str = target.to_string();

    let sessions =
        voice_session_repo::find_by_user_guild(pool, &user_id_str, &guild_id.to_string(), 20)
            .await?;
    let session =
        crate::permissions::visible_voice_sessions(ctx, guild_id, interaction.user.id, sessions)
            .await?
            .into_iter()
            .next();
    match session {
        Some(s) => {
            let embed = CreateEmbed::new()
                .title("Last Voice Session")
                .colour(Colour::new(0x2B2D31))
                .field("Channel", &s.channel_name, true)
                .field("Duration", s.duration_formatted(), true)
                .field(
                    "Joined",
                    s.joined_at.format("%Y-%m-%d %H:%M").to_string(),
                    true,
                )
                .field(
                    "Left",
                    s.left_at
                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Active".into()),
                    true,
                );
            let (embed, attachment) =
                crate::asset_manager::prepare_embed(ctx, "lastcall", embed).await;
            let mut msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(true);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            interaction
                .create_response(ctx, CreateInteractionResponse::Message(msg))
                .await?;
        }
        None => {
            let embed = embeds::info(
                "No Sessions",
                &format!("<@{}> has no voice sessions recorded.", target),
            );
            let (embed, attachment) =
                crate::asset_manager::prepare_embed(ctx, "lastcall", embed).await;
            let mut msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(true);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            interaction
                .create_response(ctx, CreateInteractionResponse::Message(msg))
                .await?;
        }
    }
    Ok(())
}
