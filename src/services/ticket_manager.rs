use std::collections::HashMap;
use std::sync::LazyLock;

use serenity::all::*;
use tracing::error;

use crate::errors::Result;
use crate::models::Guild;

static OPEN_TICKETS: LazyLock<tokio::sync::Mutex<HashMap<u64, u64>>> = LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

pub async fn send_ticket_panel(ctx: &Context, channel_id: ChannelId) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Suporte")
        .description("Clique no botão abaixo para abrir um ticket de suporte.")
        .colour(Colour::new(0x3498DB));

    let button = CreateButton::new("ticket_open")
        .label("REQUISITAR SUPORTE")
        .style(ButtonStyle::Primary);
    let row = CreateActionRow::Buttons(vec![button]);

    channel_id.send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row])).await?;
    Ok(())
}

pub async fn handle_ticket_open(ctx: &Context, interaction: &ComponentInteraction, guild_config: &Guild) -> Result<()> {
    let user_id = interaction.user.id.get();

    {
        let tickets = OPEN_TICKETS.lock().await;
        if tickets.contains_key(&user_id) {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Você já possui um ticket aberto.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    }

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

    let builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .permissions(overwrites);

    let channel = guild_id.create_channel(&ctx.http, builder).await?;

    {
        let mut tickets = OPEN_TICKETS.lock().await;
        tickets.insert(user_id, channel.id.get());
    }

    let embed = CreateEmbed::new()
        .title("Ticket Aberto")
        .description(format!("Bem-vindo ao seu ticket, <@{}>!\nDescreva seu problema e aguarde um membro da staff.", user_id))
        .colour(Colour::new(0x00FF00));

    let close_btn = CreateButton::new("ticket_close")
        .label("Fechar Ticket")
        .style(ButtonStyle::Danger);
    let row = CreateActionRow::Buttons(vec![close_btn]);

    channel.send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row])).await?;

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("Ticket criado: <#{}>", channel.id.get()))
            .ephemeral(true)
    )).await?;

    Ok(())
}

pub async fn handle_ticket_close_request(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Fechar Ticket")
        .description("Tem certeza que deseja fechar este ticket?")
        .colour(Colour::new(0xFF0000));

    let confirm_btn = CreateButton::new("ticket_close_confirm")
        .label("Sim, Fechar")
        .style(ButtonStyle::Danger);
    let cancel_btn = CreateButton::new("ticket_close_cancel")
        .label("Cancelar")
        .style(ButtonStyle::Secondary);
    let row = CreateActionRow::Buttons(vec![confirm_btn, cancel_btn]);

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![row])
            .ephemeral(true)
    )).await?;

    Ok(())
}

pub async fn handle_ticket_close_confirm(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    let channel_id = interaction.channel_id;
    let user_id = interaction.user.id.get();

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("O ticket será fechado em 5 segundos...")
            .ephemeral(true)
    )).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    {
        let mut tickets = OPEN_TICKETS.lock().await;
        tickets.remove(&user_id);
    }

    if let Err(e) = channel_id.delete(&ctx.http).await {
        error!("Failed to delete ticket channel {}: {}", channel_id, e);
    }

    Ok(())
}
