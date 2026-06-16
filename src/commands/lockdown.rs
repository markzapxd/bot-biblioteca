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
    let member = interaction.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let embed = crate::theme::info("Lockdown", "Selecione uma ação abaixo:");
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "lockdown", embed).await;

    let buttons = vec![
        CreateButton::new("lockdown_full_setup")
            .label("Lockdown + Verificação")
            .style(ButtonStyle::Danger),
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
    permissions::require_admin(user_id, member)?;
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    // 1. Criar cargos
    let member_role = guild_id.create_role(&ctx, EditRole::new().name(".").colour(Colour::new(0xFFFFFF))).await?;
    let staff_role = guild_id.create_role(&ctx, EditRole::new().name(".").colour(Colour::new(0x000001))).await?;

    // 2. Criar canais
    let mut verify_channel = guild_id.create_channel(&ctx, CreateChannel::new("verify").kind(ChannelType::Text)).await?;
    let mut staff_channel = guild_id.create_channel(&ctx, CreateChannel::new("segregação").kind(ChannelType::Text)).await?;

    // 3. Salvar no config (para o toggle usar depois)
    guild_repo::upsert(pool, &guild_id_str).await?;
    guild_repo::update_field(pool, &guild_id_str, "member_role_id", &member_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_role_id", &staff_role.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "welcome_channel_id", &verify_channel.id.to_string()).await?;
    guild_repo::update_field(pool, &guild_id_str, "staff_channel_id", &staff_channel.id.to_string()).await?;

    // Re-read config to get admin_role_id (must be set via /setup beforehand)
    let guild_config = guild_repo::find_by_id(pool, &guild_id_str).await?
        .ok_or(BotError::Validation("Guild config not found".into()))?;

    let admin_role_id = guild_config.admin_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    // 4. Trancar todos os canais de texto — só member_role + admin podem ver
    let channels = guild_id.channels(&ctx.http).await.unwrap_or_default();
    let everyone_role_id = guild_id.everyone_role();
    let mut text_count = 0u32;

    for (_, mut channel) in channels {
        if channel.kind != ChannelType::Text {
            continue;
        }

        let mut overwrites = Vec::new();

        // @everyone: negar view
        overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone_role_id),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        });

        // member_role: permitir view + send
        overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(member_role.id),
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS,
            deny: Permissions::empty(),
        });

        // admin_role: permitir tudo
        if let Some(admin_rid) = admin_role_id {
            overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(admin_rid),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            });
        }

        if let Err(e) = channel.edit(&ctx.http, EditChannel::new().permissions(overwrites)).await {
            error!("Erro ao trancar canal {}: {}", channel.id, e);
        } else {
            text_count += 1;
        }
    }

    // 5. Configurar permissões do canal verify
    //    - @everyone: pode ver (pra quem NÃO tem member_role)
    //    - member_role: negado (já está verificado)
    //    - admin: full access
    {
        let mut verify_overwrites = Vec::new();

        verify_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone_role_id),
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::empty(),
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
            error!("Erro ao configurar canal verify: {}", e);
        }
    }

    // 6. Configurar permissões do canal segregação
    //    - @everyone: negado
    //    - staff_role: full access
    //    - admin: full access
    {
        let mut staff_overwrites = Vec::new();

        staff_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone_role_id),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        });

        staff_overwrites.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(staff_role.id),
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
        });

        if let Some(admin_rid) = admin_role_id {
            staff_overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(admin_rid),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            });
        }

        if let Err(e) = staff_channel.edit(&ctx.http, EditChannel::new().permissions(staff_overwrites)).await {
            error!("Erro ao configurar canal segregação: {}", e);
        }
    }

    // 7. Postar painel de verificação no canal verify
    {
        let panel_embed = CreateEmbed::new()
            .title("VERIFICAÇÃO")
            .description("Clique no botão abaixo para iniciar seu formulário de entrada.")
            .colour(Colour::new(0x2B2D31));

        let (panel_embed, attachment) = crate::asset_manager::prepare_embed_large(ctx, "verification", panel_embed).await;

        let button = CreateButton::new("request_access")
            .label("Verificar")
            .style(ButtonStyle::Success)
            .emoji(ReactionType::Unicode("✅".into()));

        let panel_row = CreateActionRow::Buttons(vec![button]);
        let mut panel_msg = CreateMessage::new().embed(panel_embed).components(vec![panel_row]);
        if let Some(file) = attachment {
            panel_msg = panel_msg.add_file(file);
        }
        let _ = verify_channel.send_message(&ctx.http, panel_msg).await;
    }

    // 8. Resposta de sucesso
    let embed = crate::theme::success(
        "Lockdown + Configuração de Verificação",
        &format!(
            "Cargos e canais criados com sucesso!\n\
             Cargo de membro: <@&{}>\n\
             Cargo de staff: <@&{}>\n\
             Canal de verificação: <#{}>\n\
             Canal da staff: <#{}>\n\
             {} canais de texto trancados.",
            member_role.id, staff_role.id, verify_channel.id, staff_channel.id, text_count,
        ),
    );
    component.edit_response(&ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let member = component.member.as_ref().ok_or(BotError::Validation("Guild only".into()))?;
    let user_id = component.user.id.get();
    permissions::require_admin(user_id, member)?;
    let guild_id = component.guild_id.ok_or(BotError::Validation("Guild only".into()))?;

    component.create_response(&ctx, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let guild_config = match guild_repo::find_by_id(pool, &guild_id.to_string()).await? {
        Some(config) => config,
        None => return Err(BotError::Validation("Guild config not found".into())),
    };

    let channels = guild_id.channels(&ctx.http).await.unwrap_or_default();
    let everyone_role_id = guild_id.everyone_role();

    let member_role_id = guild_config.member_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    let admin_role_id = guild_config.admin_role_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(RoleId::new);

    let verify_channel_id = guild_config.welcome_channel_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new);

    let staff_channel_id = guild_config.staff_channel_id.as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(ChannelId::new);

    let first_text = channels.values().find(|c| c.kind == ChannelType::Text);
    let currently_locked = first_text.map_or(false, |ch| {
        ch.permission_overwrites.iter().any(|o| {
            matches!(o.kind, PermissionOverwriteType::Role(rid) if rid == everyone_role_id)
                && o.deny.contains(Permissions::VIEW_CHANNEL)
        })
    });

    let mut count = 0u32;

    for (_, mut channel) in channels {
        if channel.kind != ChannelType::Text {
            continue;
        }

        let is_verify = verify_channel_id.map_or(false, |id| channel.id == id);
        let _is_staff = staff_channel_id.map_or(false, |id| channel.id == id);

        let mut overwrites = channel.permission_overwrites.clone();

        if currently_locked {
            for o in &mut overwrites {
                match o.kind {
                    PermissionOverwriteType::Role(rid) if rid == everyone_role_id => {
                        o.deny.remove(Permissions::VIEW_CHANNEL);
                        o.allow |= Permissions::VIEW_CHANNEL;
                    }
                    PermissionOverwriteType::Role(rid) => {
                        if member_role_id.map_or(false, |m| rid == m) {
                            o.deny.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS);
                        }
                        if admin_role_id.map_or(false, |a| rid == a) {
                            o.deny.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS);
                        }
                    }
                    _ => {}
                }
            }
        } else {
            let mut everyone_found = false;
            let mut member_found = false;
            let mut admin_found = false;

            for o in &mut overwrites {
                match o.kind {
                    PermissionOverwriteType::Role(rid) if rid == everyone_role_id => {
                        everyone_found = true;
                        if is_verify {
                            o.allow |= Permissions::VIEW_CHANNEL;
                            o.deny.remove(Permissions::VIEW_CHANNEL);
                        } else {
                            o.deny |= Permissions::VIEW_CHANNEL;
                            o.allow.remove(Permissions::VIEW_CHANNEL);
                        }
                    }
                    PermissionOverwriteType::Role(rid) => {
                        if member_role_id.map_or(false, |m| rid == m) {
                            member_found = true;
                            if is_verify {
                                o.deny |= Permissions::VIEW_CHANNEL;
                                o.allow.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS);
                            } else {
                                o.allow |= Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS;
                                o.deny.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS);
                            }
                        }
                        if admin_role_id.map_or(false, |a| rid == a) {
                            admin_found = true;
                            o.allow |= Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS;
                            o.deny.remove(Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS);
                        }
                    }
                    _ => {}
                }
            }

            if !everyone_found {
                let deny = if is_verify { Permissions::empty() } else { Permissions::VIEW_CHANNEL };
                let allow = if is_verify { Permissions::VIEW_CHANNEL } else { Permissions::empty() };
                overwrites.push(PermissionOverwrite {
                    kind: PermissionOverwriteType::Role(everyone_role_id),
                    allow,
                    deny,
                });
            }

            if let Some(role_id) = member_role_id {
                if !member_found {
                    if is_verify {
                        overwrites.push(PermissionOverwrite {
                            kind: PermissionOverwriteType::Role(role_id),
                            allow: Permissions::empty(),
                            deny: Permissions::VIEW_CHANNEL,
                        });
                    } else {
                        overwrites.push(PermissionOverwrite {
                            kind: PermissionOverwriteType::Role(role_id),
                            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS,
                            deny: Permissions::empty(),
                        });
                    }
                }
            }

            if let Some(role_id) = admin_role_id {
                if !admin_found {
                    overwrites.push(PermissionOverwrite {
                        kind: PermissionOverwriteType::Role(role_id),
                        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS,
                        deny: Permissions::empty(),
                    });
                }
            }
        }

        if let Err(e) = channel.edit(&ctx.http, EditChannel::new().permissions(overwrites)).await {
            error!("Failed to edit channel {} permissions: {}", channel.id, e);
        } else {
            count += 1;
        }
    }

    let embed = if currently_locked {
        crate::theme::success("Lockdown Desativado", &format!("{} canais destrancados.", count))
    } else {
        crate::theme::success(
            "Lockdown Ativado",
            &format!("{} canais ajustados. Apenas membros verificados e admins podem acessar.", count),
        )
    };

    component.edit_response(&ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
