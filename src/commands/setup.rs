use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::repositories::guild_repo;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("setup")
            .description("Auto-setup bot systems")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let _guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = theme::info("Setup", "Selecione uma acao abaixo:");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let row = build_setup_row();

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_verificacao(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let member_role = guild_id.create_role(&ctx, EditRole::new().name("Verificado")).await?;
    let staff_role = guild_id.create_role(&ctx, EditRole::new().name("Staff")).await?;
    let verify_channel = guild_id.create_channel(&ctx, CreateChannel::new("verificacao").kind(ChannelType::Text)).await?;
    let staff_channel = guild_id.create_channel(&ctx, CreateChannel::new("staff").kind(ChannelType::Text)).await?;

    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, "member_role_id", &member_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_role_id", &staff_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "welcome_channel_id", &verify_channel.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_channel_id", &staff_channel.id.to_string()).await?;

    let embed = theme::success("Verification Setup Complete", &format!(
        "Member role: <@&{}>\nStaff role: <@&{}>\nVerification channel: <#{}>\nStaff channel: <#{}>",
        member_role.id, staff_role.id, verify_channel.id, staff_channel.id,
    ));
    component.edit_response(&ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}

pub async fn handle_logs(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let log_channel = guild_id.create_channel(&ctx, CreateChannel::new("logs").kind(ChannelType::Text)).await?;
    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, "log_channel_id", &log_channel.id.to_string()).await?;

    let embed = theme::success("Logs Setup Complete", &format!("Log channel: <#{}>", log_channel.id));
    component.edit_response(&ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}

pub async fn handle_setrole_click(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let options = vec![
        CreateSelectMenuOption::new("Membro", "member"),
        CreateSelectMenuOption::new("Staff", "staff"),
        CreateSelectMenuOption::new("Admin", "admin"),
    ];
    let select = CreateSelectMenu::new(
        "setup_roletype_select",
        CreateSelectMenuKind::String { options },
    ).placeholder("Selecione o tipo de cargo...");

    let embed = theme::info("Definir Cargo", "Selecione o tipo de cargo primeiro.");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![CreateActionRow::SelectMenu(select)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_setrole_type_select(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let role_type = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.first().ok_or(crate::errors::BotError::Validation("No selection".into()))?,
        _ => return Err(crate::errors::BotError::Validation("Unexpected component type".into())),
    };
    let custom_id = format!("setup_roleselect_{}", role_type);

    let role_select = CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::Role { default_roles: None },
    ).placeholder("Selecione o cargo...");

    let embed = theme::info("Definir Cargo", "Selecione o cargo desejado.");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![CreateActionRow::SelectMenu(role_select)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_role_select(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let custom_id = &component.data.custom_id;
    let role_type = custom_id.strip_prefix("setup_roleselect_")
        .ok_or_else(|| crate::errors::BotError::Validation("Invalid custom id".into()))?;

    let role_id = match &component.data.kind {
        ComponentInteractionDataKind::RoleSelect { values } => values.first().ok_or(crate::errors::BotError::Validation("Role required".into()))?,
        _ => return Err(crate::errors::BotError::Validation("Unexpected component type".into())),
    };
    let role_id = role_id.get();

    let field = match role_type {
        "member" => "member_role_id",
        "staff" => "staff_role_id",
        "admin" => "admin_role_id",
        _ => return Err(crate::errors::BotError::Validation("Invalid role type".into())),
    };

    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, field, &role_id.to_string()).await?;

    let embed = theme::success("Role Set", &format!("`{}` set to <@&{}>", role_type, role_id));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let row = build_setup_row();

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_setchannel_click(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let options = vec![
        CreateSelectMenuOption::new("Staff", "staff"),
        CreateSelectMenuOption::new("Boas-vindas", "welcome"),
        CreateSelectMenuOption::new("Logs", "log"),
    ];
    let select = CreateSelectMenu::new(
        "setup_channeltype_select",
        CreateSelectMenuKind::String { options },
    ).placeholder("Selecione o tipo de canal...");

    let embed = theme::info("Definir Canal", "Selecione o tipo de canal primeiro.");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![CreateActionRow::SelectMenu(select)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_setchannel_type_select(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let channel_type = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.first().ok_or(crate::errors::BotError::Validation("No selection".into()))?,
        _ => return Err(crate::errors::BotError::Validation("Unexpected component type".into())),
    };
    let custom_id = format!("setup_channelsel_{}", channel_type);

    let channel_select = CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::Channel { channel_types: None, default_channels: None },
    ).placeholder("Selecione o canal...");

    let embed = theme::info("Definir Canal", "Selecione o canal desejado.");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![CreateActionRow::SelectMenu(channel_select)]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_channel_select(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let custom_id = &component.data.custom_id;
    let channel_type = custom_id.strip_prefix("setup_channelsel_")
        .ok_or_else(|| crate::errors::BotError::Validation("Invalid custom id".into()))?;

    let channel_id = match &component.data.kind {
        ComponentInteractionDataKind::ChannelSelect { values } => values.first().ok_or(crate::errors::BotError::Validation("Channel required".into()))?,
        _ => return Err(crate::errors::BotError::Validation("Unexpected component type".into())),
    };
    let channel_id = channel_id.get();

    let field = match channel_type {
        "staff" => "staff_channel_id",
        "welcome" => "welcome_channel_id",
        "log" => "log_channel_id",
        _ => return Err(crate::errors::BotError::Validation("Invalid channel type".into())),
    };

    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, field, &channel_id.to_string()).await?;

    let embed = theme::success("Channel Set", &format!("`{}` set to <#{}>", channel_type, channel_id));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "setup", embed).await;
    let row = build_setup_row();

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

fn build_setup_row() -> CreateActionRow {
    let buttons = vec![
        CreateButton::new("setup_verificacao")
            .label("Setup Verification")
            .style(ButtonStyle::Primary),
        CreateButton::new("setup_logs")
            .label("Setup Logs")
            .style(ButtonStyle::Primary),
        CreateButton::new("setup_setrole")
            .label("Definir Cargo")
            .style(ButtonStyle::Secondary),
        CreateButton::new("setup_setchannel")
            .label("Definir Canal")
            .style(ButtonStyle::Secondary),
    ];
    CreateActionRow::Buttons(buttons)
}
