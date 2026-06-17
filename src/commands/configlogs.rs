use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::repositories::guild_repo;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("configlogs")
            .description("Configura as funções de logs do servidor")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();

    let guild_id_str = guild_id.to_string();
    let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &config)?;
    let modules = config.get_modules();

    let embed = build_logs_embed(&config, &modules);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "default", embed).await;
    let rows = build_logs_components(&config, &modules);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(rows);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
    let mut modules = config.get_modules();

    let selected_value = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            values.first().ok_or_else(|| crate::errors::BotError::Validation("Nenhuma opção selecionada".into()))?
        }
        _ => return Err(crate::errors::BotError::Validation("Tipo de componente inválido".into())),
    };

    match selected_value.as_str() {
        "log_calls" => modules.log_calls = !modules.log_calls,
        "log_joins_leaves" => modules.log_joins_leaves = !modules.log_joins_leaves,
        "log_roles" => modules.log_roles = !modules.log_roles,
        "log_messages" => modules.log_messages = !modules.log_messages,
        _ => return Err(crate::errors::BotError::Validation("Configuração desconhecida".into())),
    }

    let modules_json = serde_json::to_value(&modules)?;
    guild_repo::update_modules(pool, &guild_id_str, modules_json.clone()).await?;

    // Update Cache
    let mut updated_config = config.clone();
    updated_config.modules = modules_json;
    guild_cache.set(guild_id_str, updated_config.clone());

    let embed = build_logs_embed(&updated_config, &modules);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "default", embed).await;
    let rows = build_logs_components(&updated_config, &modules);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(rows);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

pub async fn handle_create_channel(ctx: &Context, component: &ComponentInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let admin_role_id = config.admin_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    let staff_role_id = config.staff_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    let everyone_role_id = guild_id.everyone_role();

    // Create channel named "logs"
    let mut channel = guild_id.create_channel(&ctx, CreateChannel::new("logs").kind(ChannelType::Text)).await?;

    let mut overwrites = Vec::new();

    // @everyone: negar view
    overwrites.push(PermissionOverwrite {
        kind: PermissionOverwriteType::Role(everyone_role_id),
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
    });

    // admin: full access
    if let Some(admin_rid) = admin_role_id {
        overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(admin_rid),
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_MESSAGES,
            deny: Permissions::empty(),
        });
    }

    // staff: read access
    if let Some(staff_rid) = staff_role_id {
        overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(staff_rid),
            allow: Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
        });
    }

    if let Err(e) = channel.edit(&ctx.http, EditChannel::new().permissions(overwrites)).await {
        tracing::error!("Erro ao configurar permissões do canal de logs: {}", e);
    }

    // Save channel ID in database
    guild_repo::update_field(pool, &guild_id_str, "log_channel_id", &channel.id.to_string()).await?;

    // Update Cache
    let mut updated_config = config.clone();
    updated_config.log_channel_id = Some(channel.id.to_string());
    guild_cache.set(guild_id_str, updated_config.clone());

    let modules = updated_config.get_modules();
    let embed = build_logs_embed(&updated_config, &modules);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "default", embed).await;
    let rows = build_logs_components(&updated_config, &modules);

    let mut msg = EditInteractionResponse::new().embed(embed).components(rows);
    if let Some(file) = attachment {
        msg = msg.new_attachment(file);
    }
    component.edit_response(&ctx, msg).await?;
    Ok(())
}

fn build_logs_embed(config: &crate::models::Guild, modules: &crate::models::guild::GuildModules) -> CreateEmbed {
    let mut desc = String::new();

    if let Some(log_channel_id) = &config.log_channel_id {
        desc.push_str(&format!("📍 **Canal de Logs**: <#{}>\n\n", log_channel_id));
        desc.push_str("Selecione uma opção no menu abaixo para ativar ou desativar uma função de log específica.\n\n");

        let status_calls = if modules.log_calls { "🟢 **ATIVADO**" } else { "🔴 **DESATIVADO**" };
        let status_joins = if modules.log_joins_leaves { "🟢 **ATIVADO**" } else { "🔴 **DESATIVADO**" };
        let status_roles = if modules.log_roles { "🟢 **ATIVADO**" } else { "🔴 **DESATIVADO**" };
        let status_messages = if modules.log_messages { "🟢 **ATIVADO**" } else { "🔴 **DESATIVADO**" };

        desc.push_str(&format!("📞 **Logs de Call**: {}\n", status_calls));
        desc.push_str(&format!("🚪 **Logs de Entradas/Saídas**: {}\n", status_joins));
        desc.push_str(&format!("🛡️ **Logs de Cargos**: {}\n", status_roles));
        desc.push_str(&format!("💬 **Logs de Mensagens**: {}\n", status_messages));
    } else {
        desc.push_str("⚠️ **Canal de Logs Não Configurado!**\n");
        desc.push_str("Nenhuma mensagem de log será enviada até que o canal de logs seja configurado.\n\n");
        desc.push_str("Clique no botão abaixo para criar e configurar o canal de logs automaticamente.");
    }

    theme::info("Painel de Logs 📝", &desc)
}

fn build_logs_components(config: &crate::models::Guild, _modules: &crate::models::guild::GuildModules) -> Vec<CreateActionRow> {
    let mut rows = Vec::new();

    if config.log_channel_id.is_some() {
        // String Select Menu
        let options = vec![
            CreateSelectMenuOption::new("Logs de Call", "log_calls")
                .description("Registra entradas/saídas em canais de voz")
                .emoji(ReactionType::Unicode("📞".to_string())),
            CreateSelectMenuOption::new("Logs de Entradas/Saídas", "log_joins_leaves")
                .description("Registra novos membros e saídas")
                .emoji(ReactionType::Unicode("🚪".to_string())),
            CreateSelectMenuOption::new("Logs de Cargos", "log_roles")
                .description("Registra alterações de cargos de membros")
                .emoji(ReactionType::Unicode("🛡️".to_string())),
            CreateSelectMenuOption::new("Logs de Mensagens", "log_messages")
                .description("Registra edições e exclusões de mensagens")
                .emoji(ReactionType::Unicode("💬".to_string())),
        ];

        rows.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "logs_select_menu",
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Selecione um log para alterar"),
        ));

        // Recriar Canal Button
        rows.push(CreateActionRow::Buttons(vec![
            CreateButton::new("logs_create_channel")
                .label("Recriar Canal de Logs")
                .style(ButtonStyle::Danger)
        ]));
    } else {
        // Criar Canal Button
        rows.push(CreateActionRow::Buttons(vec![
            CreateButton::new("logs_create_channel")
                .label("Criar Canal de Logs")
                .style(ButtonStyle::Success)
        ]));
    }

    rows
}
