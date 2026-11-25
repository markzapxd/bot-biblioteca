use serenity::all::*;
use sqlx::PgPool;
use tracing::error;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;
use crate::repositories::guild_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("lockdown")
            .description("Lockdown completo do servidor com verificação")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    let guild_config = _guild_cache.get(&guild_id.to_string())
        .ok_or_else(|| BotError::NotFound("Guild config not found".into()))?;
    permissions::require_admin(user_id, member, &guild_config)?;

    let embed = crate::theme::info("Lockdown", "Selecione uma ação abaixo:");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "lockdown", embed).await;

    let buttons = vec![
        CreateButton::new("lockdown_full_setup")
            .label("Lockdown + Verificação")
            .style(ButtonStyle::Secondary),
        CreateButton::new("lockdown_toggle")
            .label("Ativar/Desativar Lockdown")
            .style(ButtonStyle::Secondary),
    ];
    let row = CreateActionRow::Buttons(buttons);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]).ephemeral(true);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_full_setup(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let member = component.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = component.user.id.get();
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    
    let guild_config = guild_repo::find_by_id(pool, &guild_id_str).await?
        .unwrap_or_else(|| crate::models::Guild {
            guild_id: guild_id_str.clone(),
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
    permissions::require_admin(user_id, member, &guild_config)?;

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    
    let member_role = guild_id.create_role(&ctx, EditRole::new().name(".").colour(Colour::new(0xFFFFFF))).await?;
    let staff_role = guild_id.create_role(&ctx, EditRole::new().name(".").colour(Colour::new(0x000001))).await?;

    
    let mut verify_channel = guild_id.create_channel(&ctx, CreateChannel::new("verify").kind(ChannelType::Text).permissions(vec![PermissionOverwrite { kind: PermissionOverwriteType::Role(guild_id.everyone_role()), allow: Permissions::empty(), deny: Permissions::VIEW_CHANNEL }])).await?;
    let mut staff_channel = guild_id.create_channel(&ctx, CreateChannel::new("segregação").kind(ChannelType::Text).permissions(vec![PermissionOverwrite { kind: PermissionOverwriteType::Role(guild_id.everyone_role()), allow: Permissions::empty(), deny: Permissions::VIEW_CHANNEL }])).await?;

    
    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, "member_role_id", &member_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_role_id", &staff_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "welcome_channel_id", &verify_channel.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_channel_id", &staff_channel.id.to_string()).await?;

    
    let guild_config = guild_repo::find_by_id(pool, &guild_id_str).await?
        .ok_or(BotError::Validation("Guild config not found".into()))?;

    let admin_role_id = guild_config.admin_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    
    let channels = guild_id.channels(&ctx.http).await.unwrap_or_default();
    let everyone_role_id = guild_id.everyone_role();
    let mut channel_count = 0u32;

    let mut skipped_categories = std::collections::HashSet::new();
    if let Some(ch) = channels.get(&verify_channel.id) {
        if let Some(parent) = ch.parent_id {
            skipped_categories.insert(parent);
        }
    }
    if let Some(ch) = channels.get(&staff_channel.id) {
        if let Some(parent) = ch.parent_id {
            skipped_categories.insert(parent);
        }
    }

    let everyone_permissions = guild_id.to_partial_guild(&ctx.http).await?.roles.get(&everyone_role_id).map(|r|r.permissions).unwrap_or_default();
    let public: std::collections::HashSet<_> = channels.values().filter(|ch| {
        let visible = |overwrites: &[PermissionOverwrite]| {
            let mut p = everyone_permissions;
            if let Some(o) = overwrites.iter().find(|o|o.kind == PermissionOverwriteType::Role(everyone_role_id)) { p.remove(o.deny); p.insert(o.allow); }
            p.contains(Permissions::VIEW_CHANNEL)
        };
        visible(&ch.permission_overwrites) && ch.parent_id.and_then(|id|channels.get(&id)).map_or(true, |parent|visible(&parent.permission_overwrites))
    }).map(|ch|ch.id).collect();
    for (_, mut channel) in channels {
        let should_process = match channel.kind {
            ChannelType::Category => {
                !skipped_categories.contains(&channel.id)
            }
            ChannelType::Text | ChannelType::Voice => {
                let is_verify = channel.id == verify_channel.id;
                let is_staff = channel.id == staff_channel.id;

                !(is_verify || is_staff)
            }
            _ => false,
        };

        if !should_process {
            continue;
        }

        let overwrites = crate::services::lockdown::setup_visible_permissions(&channel.permission_overwrites, everyone_role_id, member_role.id, public.contains(&channel.id));

        if let Err(e) = channel.edit(&ctx.http, EditChannel::new().permissions(overwrites)).await {
            error!("Erro ao trancar canal/categoria {}: {}", channel.id, e);
        } else {
            channel_count += 1;
        }
    }

    
    
    
    
    
    {
        let mut verify_overwrites = Vec::new();

        verify_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone_role_id),
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::SEND_MESSAGES
                | Permissions::SEND_MESSAGES_IN_THREADS
                | Permissions::CREATE_PUBLIC_THREADS
                | Permissions::CREATE_PRIVATE_THREADS
                | Permissions::ADD_REACTIONS,
        });

        verify_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(member_role.id),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        });

        if let Some(admin_rid) = admin_role_id {
            verify_overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(admin_rid),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            });
        }

        if let Err(e) = verify_channel.edit(&ctx.http, EditChannel::new().permissions(verify_overwrites)).await {
            return Err(e.into());
        }
    }

    
    
    
    
    {
        let mut staff_overwrites = Vec::new();

        staff_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone_role_id),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        });

        staff_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(staff_role.id),
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::SEND_MESSAGES
                | Permissions::SEND_MESSAGES_IN_THREADS
                | Permissions::CREATE_PUBLIC_THREADS
                | Permissions::CREATE_PRIVATE_THREADS
                | Permissions::ADD_REACTIONS,
        });

        if let Some(admin_rid) = admin_role_id {
            staff_overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(admin_rid),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            });
        }

        if let Err(e) = staff_channel.edit(&ctx.http, EditChannel::new().permissions(staff_overwrites)).await {
            return Err(e.into());
        }
    }

    
    {
        let panel_embed = CreateEmbed::new()
            .title("VERIFICAÇÃO")
            .description("Clique no botão abaixo para iniciar seu formulário de entrada.")
            .colour(Colour::new(0x2B2D31));

        let (panel_embed, attachment) = crate::asset_manager::prepare_embed_large(ctx, "verification", panel_embed).await;

        let button = CreateButton::new("request_access")
            .label("Verificar")
            .style(ButtonStyle::Secondary);

        let panel_row = CreateActionRow::Buttons(vec![button]);
        let mut panel_msg = CreateMessage::new().embed(panel_embed).components(vec![panel_row]);
        if let Some(file) = attachment {
            panel_msg = panel_msg.add_file(file);
        }
        let _ = verify_channel.send_message(&ctx.http, panel_msg).await;
    }

    
    let embed = crate::theme::success(
        "Lockdown + Configuração de Verificação",
        &format!(
            "Cargos e canais criados com sucesso!\n\
             Cargo de membro: <@&{}>\n\
             Cargo de staff: <@&{}>\n\
             Canal de verificação: <#{}>\n\
             Canal da staff: <#{}>\n\
             {} canais trancados.",
            member_role.id, staff_role.id, verify_channel.id, staff_channel.id, channel_count,
        ),
    );
    component.edit_response(&ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let member = component.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = component.user.id.get();
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let guild_config = match guild_repo::find_by_id(pool, &guild_id.to_string()).await? {
        Some(config) => config,
        None => return Err(BotError::Validation("Guild config not found".into())),
    };

    permissions::require_admin(user_id, member, &guild_config)?;

    let enabled = crate::services::lockdown::apply(ctx, pool, guild_id, &guild_config, None).await?;
    let embed = crate::theme::success("Lockdown", if enabled { "Lockdown ativado." } else { "Permissões originais restauradas." });
    component.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
