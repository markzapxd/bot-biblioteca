use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("config")
            .description("Configurar cargos e canais do servidor")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let guild_config = _guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    let embed = build_config_embed(&guild_config);
    let rows = build_main_rows();

    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows)
            .ephemeral(true)
    )).await?;
    Ok(())
}

fn build_config_embed(config: &crate::models::Guild) -> CreateEmbed {
    let mut desc = String::new();

    desc.push_str("**Cargos**\n");
    if let Some(id) = &config.admin_role_id {
        desc.push_str(&format!("Admin: <@&{}>\n", id));
    } else {
        desc.push_str("Admin: Nao configurado\n");
    }
    if let Some(id) = &config.staff_role_id {
        desc.push_str(&format!("Staff: <@&{}>\n", id));
    } else {
        desc.push_str("Staff: Nao configurado\n");
    }
    if let Some(id) = &config.member_role_id {
        desc.push_str(&format!("Verificado: <@&{}>\n", id));
    } else {
        desc.push_str("Verificado: Nao configurado\n");
    }

    desc.push_str("\n**Canais**\n");
    if let Some(id) = &config.staff_channel_id {
        desc.push_str(&format!("Staff: <#{}>\n", id));
    } else {
        desc.push_str("Staff: Nao configurado\n");
    }
    if let Some(id) = &config.welcome_channel_id {
        desc.push_str(&format!("Boas-vindas: <#{}>\n", id));
    } else {
        desc.push_str("Boas-vindas: Nao configurado\n");
    }
    if let Some(id) = &config.log_channel_id {
        desc.push_str(&format!("Logs: <#{}>\n", id));
    } else {
        desc.push_str("Logs: Nao configurado\n");
    }
    if let Some(id) = &config.ticket_category_id {
        desc.push_str(&format!("Tickets: <#{}>\n", id));
    } else {
        desc.push_str("Tickets: Nao configurado\n");
    }

    CreateEmbed::new()
        .title("Configuracao do Servidor")
        .description(desc)
        .colour(Colour::new(0x2B2D31))
}

fn build_main_rows() -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("cfg_roles")
                .label("Cargos")
                .style(ButtonStyle::Primary),
            CreateButton::new("cfg_channels")
                .label("Canais")
                .style(ButtonStyle::Primary),
        ]),
    ]
}

fn build_roles_rows() -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_role_admin",
                CreateSelectMenuKind::Role { default_roles: None },
            )
            .placeholder("Selecione o cargo de administrador"),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_role_staff",
                CreateSelectMenuKind::Role { default_roles: None },
            )
            .placeholder("Selecione o cargo de staff"),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_role_member",
                CreateSelectMenuKind::Role { default_roles: None },
            )
            .placeholder("Selecione o cargo verificado"),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new("cfg_back")
                .label("Voltar")
                .style(ButtonStyle::Secondary),
        ]),
    ]
}

fn build_channels_rows() -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_channel_staff",
                CreateSelectMenuKind::Channel { channel_types: Some(vec![ChannelType::Text]), default_channels: None },
            )
            .placeholder("Selecione o canal de staff"),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_channel_welcome",
                CreateSelectMenuKind::Channel { channel_types: Some(vec![ChannelType::Text]), default_channels: None },
            )
            .placeholder("Selecione o canal de boas-vindas"),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_channel_logs",
                CreateSelectMenuKind::Channel { channel_types: Some(vec![ChannelType::Text]), default_channels: None },
            )
            .placeholder("Selecione o canal de logs"),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "cfg_channel_tickets",
                CreateSelectMenuKind::Channel { channel_types: Some(vec![ChannelType::Category]), default_channels: None },
            )
            .placeholder("Selecione a categoria de tickets"),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new("cfg_back")
                .label("Voltar")
                .style(ButtonStyle::Secondary),
        ]),
    ]
}

pub async fn handle_roles_button(ctx: &Context, component: &ComponentInteraction, state: &crate::state::BotState) -> Result<()> {
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;

    let embed = build_config_embed(&config);
    let rows = build_roles_rows();

    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows),
    )).await?;
    Ok(())
}

pub async fn handle_channels_button(ctx: &Context, component: &ComponentInteraction, state: &crate::state::BotState) -> Result<()> {
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;

    let embed = build_config_embed(&config);
    let rows = build_channels_rows();

    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows),
    )).await?;
    Ok(())
}

pub async fn handle_back_button(ctx: &Context, component: &ComponentInteraction, state: &crate::state::BotState) -> Result<()> {
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();
    let config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;

    let embed = build_config_embed(&config);
    let rows = build_main_rows();

    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(rows),
    )).await?;
    Ok(())
}

async fn update_role_field(
    ctx: &Context,
    component: &ComponentInteraction,
    state: &crate::state::BotState,
    field: &str,
    role_id: RoleId,
) -> Result<()> {
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    crate::repositories::guild_repo::update_field(&state.pool, &guild_id_str, field, &role_id.to_string()).await?;

    let mut config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    match field {
        "admin_role_id" => config.admin_role_id = Some(role_id.to_string()),
        "staff_role_id" => config.staff_role_id = Some(role_id.to_string()),
        "member_role_id" => config.member_role_id = Some(role_id.to_string()),
        _ => {}
    }
    state.guild_cache.set(guild_id_str.clone(), config);

    component.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;

    let embed = build_config_embed(&config);
    let rows = build_roles_rows();

    component.edit_response(&ctx.http, EditInteractionResponse::new()
        .embed(embed)
        .components(rows)
    ).await?;

    Ok(())
}

async fn update_channel_field(
    ctx: &Context,
    component: &ComponentInteraction,
    state: &crate::state::BotState,
    field: &str,
    channel_id: ChannelId,
) -> Result<()> {
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    crate::repositories::guild_repo::update_field(&state.pool, &guild_id_str, field, &channel_id.to_string()).await?;

    let mut config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    match field {
        "staff_channel_id" => config.staff_channel_id = Some(channel_id.to_string()),
        "welcome_channel_id" => config.welcome_channel_id = Some(channel_id.to_string()),
        "log_channel_id" => config.log_channel_id = Some(channel_id.to_string()),
        "ticket_category_id" => config.ticket_category_id = Some(channel_id.to_string()),
        _ => {}
    }
    state.guild_cache.set(guild_id_str.clone(), config);

    component.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let config = state.guild_cache.get(&guild_id_str)
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;

    let embed = build_config_embed(&config);
    let rows = build_channels_rows();

    component.edit_response(&ctx.http, EditInteractionResponse::new()
        .embed(embed)
        .components(rows)
    ).await?;

    Ok(())
}

pub async fn handle_role_select(ctx: &Context, component: &ComponentInteraction, state: &crate::state::BotState) -> Result<()> {
    let custom_id = &component.data.custom_id;
    let role_id = match &component.data.kind {
        ComponentInteractionDataKind::RoleSelect { values } => {
            values.first().copied().ok_or(BotError::Validation("Nenhum cargo selecionado".into()))?
        }
        _ => return Err(BotError::Validation("Tipo de componente invalido".into())),
    };

    let field = match custom_id.as_str() {
        "cfg_role_admin" => "admin_role_id",
        "cfg_role_staff" => "staff_role_id",
        "cfg_role_member" => "member_role_id",
        _ => return Ok(()),
    };

    update_role_field(ctx, component, state, field, role_id).await
}

pub async fn handle_channel_select(ctx: &Context, component: &ComponentInteraction, state: &crate::state::BotState) -> Result<()> {
    let custom_id = &component.data.custom_id;
    let channel_id = match &component.data.kind {
        ComponentInteractionDataKind::ChannelSelect { values } => {
            values.first().copied().ok_or(BotError::Validation("Nenhum canal selecionado".into()))?
        }
        _ => return Err(BotError::Validation("Tipo de componente invalido".into())),
    };

    let field = match custom_id.as_str() {
        "cfg_channel_staff" => "staff_channel_id",
        "cfg_channel_welcome" => "welcome_channel_id",
        "cfg_channel_logs" => "log_channel_id",
        "cfg_channel_tickets" => "ticket_category_id",
        _ => return Ok(()),
    };

    update_channel_field(ctx, component, state, field, channel_id).await
}
