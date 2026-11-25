use super::support_state::{self, Key};
use serenity::all::*;

use crate::errors::Result;
use crate::models::Guild;
use crate::theme;



pub async fn send_ticket_panel(ctx: &Context, channel_id: ChannelId) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Suporte")
        .description("Clique no botão abaixo para abrir um ticket de suporte.")
        .colour(Colour::new(0x2B2D31));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "ticketpanel", embed).await;

    let button = CreateButton::new("ticket_open")
        .label("REQUISITAR SUPORTE")
        .style(ButtonStyle::Secondary);
    let row = CreateActionRow::Buttons(vec![button]);

    let mut msg = CreateMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    channel_id.send_message(&ctx.http, msg).await?;
    Ok(())
}

pub async fn handle_ticket_open(ctx: &Context, interaction: &ComponentInteraction, guild_config: &Guild) -> Result<()> {
    let user_id = interaction.user.id.get();

    let guild_id = match interaction.guild_id {
        Some(gid) => gid,
        None => {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Este comando só pode ser usado em servidores.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    };
    interaction.defer_ephemeral(&ctx.http).await?;
    guild_id.member(&ctx.http, interaction.user.id).await?;
    let store = support_state::store(ctx).await?;
    let key = Key::new(guild_id.get(), user_id);
    if !store.reserve_ticket(key).await? {
        interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Você já possui um ticket aberto ou em criação.")).await?;
        return Ok(());
    }
    let username = interaction.user.name.clone();
    let channel_name = format!("ticket-{}", username.to_lowercase().replace(' ', "-"));

    let mut overwrites = vec![
        PermissionOverwrite {
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        },
        PermissionOverwrite {
            kind: PermissionOverwriteType::Member(interaction.user.id),
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
        },
    ];

    if let Some(staff_role_id) = &guild_config.staff_role_id {
        if let Ok(role_id_num) = staff_role_id.parse::<u64>() {
            overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(RoleId::new(role_id_num)),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                deny: Permissions::empty(),
            });
        }
    }

    if let Some(admin_role_id) = &guild_config.admin_role_id {
        if let Ok(role_id_num) = admin_role_id.parse::<u64>() {
            overwrites.push(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(RoleId::new(role_id_num)),
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            });
        }
    }

    let builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .permissions(overwrites);

    let channel = match guild_id.create_channel(&ctx.http, builder).await {
        Ok(channel) => channel,
        Err(error) => {
            store.release_ticket(key).await?;
            interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Falha ao criar ticket. Tente novamente.")).await?;
            return Err(error.into());
        }
    };
    store.attach_ticket(key, channel.id.get()).await?;

    let embed = CreateEmbed::new()
        .title("Ticket Aberto")
        .description(format!("Bem-vindo ao seu ticket, <@{}>!\nDescreva seu problema e aguarde um membro da staff.", user_id))
        .colour(Colour::new(0x2B2D31));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "ticket", embed).await;

    let close_btn = CreateButton::new("ticket_close")
        .label("Fechar Ticket")
        .style(ButtonStyle::Secondary);
    let row = CreateActionRow::Buttons(vec![close_btn]);

    let mut msg = CreateMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    channel.send_message(&ctx.http, msg).await?;

    interaction.edit_response(&ctx.http, EditInteractionResponse::new()
        .content(format!("Ticket criado: <#{}>", channel.id.get()))).await?;

    Ok(())
}

async fn close_policy(ctx: &Context, guild: GuildId, channel: ChannelId, user: UserId) -> Result<bool> {
    let store = support_state::store(ctx).await?;
    let owner = store.ticket_owner(guild.get(), channel.get()).await?
        .ok_or_else(|| crate::errors::BotError::Unauthorized("Canal não é um ticket registrado".into()))?;
    let config = {
        let data = ctx.data.read().await;
        let state = data.get::<crate::state::BotStateKey>().ok_or_else(|| crate::errors::BotError::Internal("State unavailable".into()))?;
        crate::repositories::guild_repo::find_by_id(&state.pool, &guild.to_string()).await?
            .ok_or_else(|| crate::errors::BotError::Unauthorized("Configuração ausente".into()))?
    };
    let staff = support_state::staff(ctx, guild, user, &config).await?;
    if owner.user != user.get() && !staff {
        return Err(crate::errors::BotError::Unauthorized("Apenas dono ou staff".into()));
    }
    Ok(staff)
}

pub async fn handle_ticket_close_request(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    interaction.defer_ephemeral(&ctx.http).await?;
    let guild = interaction.guild_id.ok_or_else(|| crate::errors::BotError::Unauthorized("Guild only".into()))?;
    close_policy(ctx, guild, interaction.channel_id, interaction.user.id).await?;
    interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Tem certeza que deseja fechar este ticket?")
        .components(vec![CreateActionRow::Buttons(vec![
            CreateButton::new("ticket_close_confirm").label("Sim, Fechar").style(ButtonStyle::Danger),
            CreateButton::new("ticket_close_cancel").label("Cancelar").style(ButtonStyle::Secondary),
        ])])).await?;
    Ok(())
}

pub async fn handle_ticket_close_cancel(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().content("Fechamento cancelado.").components(vec![]))).await?;
    Ok(())
}

pub async fn handle_ticket_close_confirm(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    interaction.defer_ephemeral(&ctx.http).await?;
    let guild = interaction.guild_id.ok_or_else(|| crate::errors::BotError::Unauthorized("Guild only".into()))?;
    let staff = close_policy(ctx, guild, interaction.channel_id, interaction.user.id).await?;
    let store = support_state::store(ctx).await?;
    let result = store.close_ticket(guild.get(), interaction.channel_id.get(), interaction.user.id.get(), staff,
        || async { interaction.channel_id.delete(&ctx.http).await?; Ok(()) }).await;
    let text = match &result { Ok(true) => "Ticket fechado.", Ok(false) => "Ticket não encontrado.", Err(_) => "Falha ao excluir ticket. Tente novamente." };
    interaction.edit_response(&ctx.http, EditInteractionResponse::new().content(text).components(vec![])).await?;
    result?;
    Ok(())
}

pub async fn handle_close_ticket_command(ctx: &Context, interaction: &CommandInteraction, target_user_id: UserId) -> Result<()> {
    interaction.defer_ephemeral(&ctx.http).await?;
    finish_deferred_close(
        close_ticket_command_deferred(ctx, interaction, target_user_id),
        || async {
            interaction.edit_response(&ctx.http, EditInteractionResponse::new()
                .content("Não foi possível fechar o ticket. Verifique sua autorização e a configuração do servidor.")).await?;
            Ok(())
        },
    ).await
}

async fn finish_deferred_close<T, F, R>(operation: T, respond: F) -> Result<()>
where
    T: std::future::Future<Output = Result<()>>,
    F: FnOnce() -> R,
    R: std::future::Future<Output = Result<()>>,
{
    let result = operation.await;
    if result.is_err() { respond().await?; }
    result
}

async fn close_ticket_command_deferred(ctx: &Context, interaction: &CommandInteraction, target_user_id: UserId) -> Result<()> {
    let guild = interaction.guild_id.ok_or_else(|| crate::errors::BotError::Unauthorized("Guild only".into()))?;
    let store = support_state::store(ctx).await?;
    let Some(channel) = store.ticket_channel(Key::new(guild.get(), target_user_id.get())).await? else {
        interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Ticket não encontrado.")).await?;
        return Ok(());
    };
    let channel = ChannelId::new(channel);
    let staff = close_policy(ctx, guild, channel, interaction.user.id).await?;
    let result = store.close_ticket(guild.get(), channel.get(), interaction.user.id.get(), staff,
        || async { channel.delete(&ctx.http).await?; Ok(()) }).await;
    let text = match &result { Ok(true) => "Ticket fechado.", Ok(false) => "Ticket não encontrado.", Err(_) => "Falha ao excluir ticket. Tente novamente." };
    interaction.edit_response(&ctx.http, EditInteractionResponse::new().content(text)).await?;
    result?;
    Ok(())
}

pub async fn handle_add_user_to_ticket(ctx: &Context, interaction: &CommandInteraction, target: UserId) -> Result<()> {
    let channel = interaction.channel_id;
    channel.create_permission(&ctx.http, PermissionOverwrite {
        kind: PermissionOverwriteType::Member(target),
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
    }).await?;

    let embed = theme::success("Usuario Adicionado", &format!("<@{}> foi adicionado ao ticket.", target.get()));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "addticket", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_remove_user_from_ticket(ctx: &Context, interaction: &CommandInteraction, target: UserId) -> Result<()> {
    let channel = interaction.channel_id;

    channel.delete_permission(&ctx.http, PermissionOverwriteType::Member(target)).await?;

    let embed = theme::success("Usuario Removido", &format!("<@{}> foi removido do ticket.", target.get()));
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "removeticket", embed).await;
    let mut msg = CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

#[cfg(test)]
mod deferred_close_tests {
    use super::*;

    #[tokio::test]
    async fn deferred_errors_receive_terminal_response() {
        for error in [
            crate::errors::BotError::Unauthorized("denied".into()),
            crate::errors::BotError::Unauthorized("configuration missing".into()),
            crate::errors::BotError::Internal("membership lookup failed".into()),
        ] {
            let called = std::cell::Cell::new(0);
            let result = finish_deferred_close(async { Err(error) }, || async {
                called.set(called.get() + 1);
                Ok(())
            }).await;
            assert!(result.is_err());
            assert_eq!(called.get(), 1);
        }
    }
}

