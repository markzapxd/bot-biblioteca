use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::services::anti_raid;
use crate::repositories::guild_repo;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("raidmode")
            .description("Manually control raid mode")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_config = _guild_cache.get(&guild_id.to_string())
        .ok_or_else(|| crate::errors::BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    let is_active = anti_raid::is_raid_active(guild_id.get());
    let embed = build_raidmode_embed(is_active);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "raidmode", embed).await;
    let row = build_raidmode_row(is_active);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;

    // Defer response immediately to avoid 3-second timeout
    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let is_active = anti_raid::is_raid_active(guild_id.get());

    if is_active {
        anti_raid::disable_raid_mode(&ctx, guild_id).await?;
    } else {
        let guild_config = guild_repo::find_by_id(pool, &guild_id.to_string()).await?.unwrap_or_else(|| default_guild(guild_id.get()));
        anti_raid::trigger_raid(&ctx, guild_id, "Manual activation by admin", &guild_config).await?;
    }

    let new_status = !is_active;
    let embed = build_raidmode_embed(new_status);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "raidmode", embed).await;
    let row = build_raidmode_row(new_status);

    let mut msg = EditInteractionResponse::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.new_attachment(file);
    }
    component.edit_response(&ctx, msg).await?;
    Ok(())
}

fn build_raidmode_embed(is_active: bool) -> CreateEmbed {
    let status = if is_active { "Ativado" } else { "Desativado" };
    theme::warning("Modo Raid", &format!("O modo raid esta **{}**.", status))
}

fn build_raidmode_row(is_active: bool) -> CreateActionRow {
    let btn = CreateButton::new("raidmode_toggle")
        .label(if is_active { "Desativar" } else { "Ativar" })
        .style(ButtonStyle::Secondary);
    CreateActionRow::Buttons(vec![btn])
}

fn default_guild(guild_id: u64) -> crate::models::Guild {
    crate::models::Guild {
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
    }
}
