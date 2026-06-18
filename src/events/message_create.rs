use serenity::all::*;
use crate::permissions;

pub async fn handle(ctx: Context, msg: Message) {
    if msg.author.bot {
        return;
    }

    if msg.content == "!owner" || msg.content.starts_with("!owner ") {
        if let Err(e) = crate::commands::owner::handle_prefix(&ctx, &msg).await {
            tracing::error!("Error handling !owner command: {:?}", e);
        }
        return;
    }

    if msg.content.starts_with("!reloadslash") {
        let _ = handle_reloadslash(&ctx, &msg).await;
        return;
    }

    if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        let pool = &state.pool;
        if let Some(guild_id) = msg.guild_id {
            let guild_id_str = guild_id.to_string();
            let guild_config = if let Some(config) = state.guild_cache.get(&guild_id_str) {
                config
            } else {
                let config = match crate::repositories::guild_repo::find_by_id(pool, &guild_id_str).await {
                    Ok(Some(c)) => c,
                    _ => {
                        match crate::repositories::guild_repo::upsert(pool, &guild_id_str).await {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::error!("Failed to upsert guild config for message: {:?}", e);
                                return;
                            }
                        }
                    }
                };
                state.guild_cache.set(guild_id_str.clone(), config.clone());
                config
            };

            let _ = crate::services::anti_raid::detect_message_violation(&ctx, &msg, &guild_config).await;
        }
    }
}

async fn handle_reloadslash(ctx: &Context, msg: &Message) -> crate::errors::Result<()> {
    let guild_id = msg.guild_id.ok_or_else(|| {
        crate::errors::BotError::Validation("Comando apenas em servidor".into())
    })?;
    let member = guild_id.member(ctx, msg.author.id).await?;
    let state = ctx.data.read().await.get::<crate::state::BotStateKey>().cloned()
        .ok_or_else(|| crate::errors::BotError::Internal("No bot state".into()))?;
    let guild_config = crate::repositories::guild_repo::find_by_id(&state.pool, &guild_id.to_string()).await?
        .ok_or_else(|| crate::errors::BotError::Validation("Guild config not found".into()))?;
    if let Err(e) = permissions::require_admin(msg.author.id.get(), &member, &guild_config) {
        let _ = msg.reply(ctx, format!("{}", e)).await;
        return Ok(());
    }

    let mut reply = msg.reply(ctx, "Apagando e re-registrando comandos...").await?;

    let guilds = ctx.cache.guilds();
    let count = guilds.len();

    let mut cmds = Vec::new();
    crate::commands::register_all(&mut cmds).await;

    for guild_id in &guilds {
        let _ = guild_id.set_commands(ctx, vec![]).await;
    }

    for guild_id in &guilds {
        let _ = guild_id.set_commands(ctx, cmds.clone()).await;
    }

    let embed = crate::theme::success(
        "Comandos Recarregados",
        &format!("Slash commands apagados e registrados dnv em {} servidores ({} comandos).", count, cmds.len()),
    );
    reply.edit(ctx, EditMessage::new().embed(embed).content("")).await?;
    Ok(())
}
