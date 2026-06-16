use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::services::user_info_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("userinfo")
            .description("Tactical user file")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;

    let target_user = target.to_user(&ctx.http).await?;
    let guild_config = crate::repositories::guild_repo::find_by_id(pool, &guild_id.to_string()).await?.unwrap_or_else(|| crate::models::Guild {
        guild_id: guild_id.to_string(),
        prefix: None,
        member_role_id: None,
        staff_channel_id: None,
        welcome_channel_id: None,
        log_channel_id: None,
        admin_role_id: None,
        staff_role_id: None,
        ticket_category_id: None,
        frin_monitor_channel_id: None,
        modules: serde_json::json!({}),
        webhook_url: None,
        premium: None,
        track_mute: None,
        track_deaf: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });
    let interaction_user_id = interaction.user.id.get();
    let is_staff = interaction.member.as_ref().map(|m| crate::permissions::is_staff(m, &guild_config)).unwrap_or(false);

    let result = user_info_manager::build_user_info(ctx, &target_user, &guild_config, interaction_user_id, is_staff, pool).await?;

    if result.restricted {
        let content = result.content.unwrap_or_else(|| "This user has privacy mode enabled.".into());
        interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content).ephemeral(true))).await?;
        return Ok(());
    }

    let mut msg = CreateInteractionResponseMessage::new();
    if let Some(embed) = result.embed {
        msg = msg.embed(embed);
    }
    if let Some(row) = result.action_row {
        msg = msg.components(vec![row]);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
