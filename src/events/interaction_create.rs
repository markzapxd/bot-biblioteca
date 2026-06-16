use serenity::all::{Context, Interaction};

pub async fn handle(ctx: Context, interaction: Interaction) {
    let state = match ctx.data.read().await.get::<crate::state::BotStateKey>() {
        Some(s) => s.clone(),
        None => return,
    };

    match interaction {
        Interaction::Command(command) => {
            tracing::info!("Received command: {}", command.data.name);
            if let Err(e) = crate::commands::route(&ctx, &command, &state).await {
                tracing::error!("Error routing command {}: {:?}", command.data.name, e);
            }
        }
        Interaction::Component(component) => {
            let custom_id = &component.data.custom_id;
            tracing::info!("Received component interaction: {}", custom_id);

            // Fetch guild config if we are in a guild
            let guild_config = if let Some(guild_id) = component.guild_id {
                match crate::repositories::guild_repo::find_by_id(&state.pool, &guild_id.to_string()).await {
                    Ok(Some(config)) => config,
                    _ => crate::models::Guild {
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
            } else {
                crate::models::Guild {
                    guild_id: "0".to_string(),
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
            };

            let result = match *custom_id {
                _ if custom_id == "privacy_toggle" => {
                    crate::commands::privacy::handle_toggle(&ctx, &component, &state.pool).await
                }
                _ if custom_id == "raidmode_toggle" => {
                    crate::commands::raidmode::handle_toggle(&ctx, &component, &state.pool).await
                }
                _ if custom_id.starts_with("modules_toggle_") => {
                    crate::commands::modulos::handle_toggle(&ctx, &component, &state.pool, &state.guild_cache).await
                }
                _ if custom_id == "lockdown_full_setup" => {
                    crate::commands::lockdown::handle_full_setup(&ctx, &component, &state.pool).await
                }
                _ if custom_id == "lockdown_toggle" => {
                    crate::commands::lockdown::handle_toggle(&ctx, &component, &state.pool).await
                }
                _ if custom_id.starts_with("avatar_") => {
                    let parts: Vec<&str> = custom_id.split('_').collect();
                    if parts.len() == 3 {
                        let user_id = parts[1].parse::<u64>().unwrap_or(0);
                        let page = parts[2].parse::<usize>().unwrap_or(0);
                        crate::services::avatar_manager::handle_avatar_history(&ctx, &component, user_id, page, &state.pool).await
                    } else {
                        Ok(())
                    }
                }
                _ if custom_id.starts_with("userinfo_back_") => {
                    crate::services::user_info_manager::handle_user_info_back(&ctx, &component, &state.pool).await
                }
                _ if custom_id == "request_access" => {
                    crate::services::member_manager::handle_access_request(&ctx, &component, &guild_config).await
                }
                _ if custom_id == "referral_select" => {
                    crate::services::member_manager::handle_referral_selection(&ctx, &component, &guild_config).await
                }
                _ if custom_id.starts_with("approve_") => {
                    crate::services::member_manager::handle_approval_action(&ctx, &component, true, &guild_config).await
                }
                _ if custom_id.starts_with("reject_") => {
                    crate::services::member_manager::handle_approval_action(&ctx, &component, false, &guild_config).await
                }
                _ if custom_id == "ticket_open" => {
                    crate::services::ticket_manager::handle_ticket_open(&ctx, &component, &guild_config).await
                }
                _ if custom_id == "ticket_close" => {
                    crate::services::ticket_manager::handle_ticket_close_request(&ctx, &component).await
                }
                _ if custom_id == "ticket_close_confirm" => {
                    crate::services::ticket_manager::handle_ticket_close_confirm(&ctx, &component).await
                }
                _ if custom_id.starts_with("lookup_ban_") => {
                    let parts: Vec<&str> = custom_id.split('_').collect();
                    if let Some(target_str) = parts.get(2) {
                        if let Ok(target_id) = target_str.parse::<u64>() {
                            let target = serenity::all::UserId::new(target_id);
                            crate::commands::lookup::handle_ban(&ctx, &component, target).await
                        } else { Ok(()) }
                    } else { Ok(()) }
                }
                _ if custom_id.starts_with("lookup_kick_") => {
                    let parts: Vec<&str> = custom_id.split('_').collect();
                    if let Some(target_str) = parts.get(2) {
                        if let Ok(target_id) = target_str.parse::<u64>() {
                            let target = serenity::all::UserId::new(target_id);
                            crate::commands::lookup::handle_kick(&ctx, &component, target).await
                        } else { Ok(()) }
                    } else { Ok(()) }
                }
                _ if custom_id.starts_with("lookup_mute_") => {
                    let parts: Vec<&str> = custom_id.split('_').collect();
                    if let Some(target_str) = parts.get(2) {
                        if let Ok(target_id) = target_str.parse::<u64>() {
                            let target = serenity::all::UserId::new(target_id);
                            crate::commands::lookup::handle_mute(&ctx, &component, target).await
                        } else { Ok(()) }
                    } else { Ok(()) }
                }
                _ if custom_id.starts_with("lookup_warn_") => {
                    let parts: Vec<&str> = custom_id.split('_').collect();
                    if let Some(target_str) = parts.get(2) {
                        if let Ok(target_id) = target_str.parse::<u64>() {
                            let target = serenity::all::UserId::new(target_id);
                            crate::commands::lookup::handle_warn(&ctx, &component, target).await
                        } else { Ok(()) }
                    } else { Ok(()) }
                }
                _ if custom_id == "help_membro" => {
                    crate::commands::help::handle_membro(&ctx, &component).await
                }
                _ if custom_id == "help_admin" => {
                    crate::commands::help::handle_admin(&ctx, &component).await
                }
                _ if custom_id == "help_admin_p1" => {
                    crate::commands::help::handle_admin_p1(&ctx, &component).await
                }
                _ if custom_id == "help_admin_p2" => {
                    crate::commands::help::handle_admin_p2(&ctx, &component).await
                }
                _ if custom_id == "help_back" => {
                    crate::commands::help::handle_back(&ctx, &component).await
                }
                _ => Ok(()),
            };

            if let Err(e) = result {
                tracing::error!("Error handling component interaction {}: {:?}", custom_id, e);
            }
        }
        _ => {}
    }
}
