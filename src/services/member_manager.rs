use std::collections::HashSet;
use std::sync::LazyLock;

use serenity::all::*;
use tracing::error;

use crate::errors::Result;
use crate::models::Guild;

static PENDING_REQUESTS: LazyLock<tokio::sync::Mutex<HashSet<u64>>> = LazyLock::new(|| tokio::sync::Mutex::new(HashSet::new()));

pub async fn handle_access_request(ctx: &Context, interaction: &ComponentInteraction, guild_config: &Guild) -> Result<()> {
    let user_id = interaction.user.id.get();
    let guild_id = interaction.guild_id.map(|g| g.get()).unwrap_or(0);

    if let Some(member) = &interaction.member {
        if let Some(role_id_str) = &guild_config.member_role_id {
            if let Ok(role_id_num) = role_id_str.parse::<u64>() {
                if member.roles.iter().any(|r| r.get() == role_id_num) {
                    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Você já possui acesso ao servidor.")
                            .ephemeral(true)
                    )).await?;
                    return Ok(());
                }
            }
        }
    }

    {
        let pending = PENDING_REQUESTS.lock().await;
        if pending.contains(&user_id) {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Você já possui uma solicitação de acesso pendente.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    }

    let menu = CreateSelectMenu::new("referral_select", CreateSelectMenuKind::User { default_users: None });
    let row = CreateActionRow::SelectMenu(menu);

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Selecione um membro da staff para referenciá-lo:")
            .components(vec![row])
            .ephemeral(true)
    )).await?;

    {
        let mut pending = PENDING_REQUESTS.lock().await;
        pending.insert(user_id);
    }

    Ok(())
}

pub async fn handle_referral_selection(ctx: &Context, interaction: &ComponentInteraction, guild_config: &Guild) -> Result<()> {
    let selected_user_id = interaction.data.values.first()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let is_staff_member = if let Some(guild_id) = interaction.guild_id {
        if let Ok(member) = guild_id.member(&ctx.http, selected_user_id).await {
            crate::permissions::is_staff(&member, guild_config)
        } else {
            false
        }
    } else {
        false
    };

    if !is_staff_member {
        interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("O usuário selecionado não é um membro da staff.")
                .ephemeral(true)
        )).await?;
        return Ok(());
    }

    let requester_id = interaction.user.id.get();
    let embed = CreateEmbed::new()
        .title("Solicitação de Acesso")
        .description(format!("<@{}> solicitou acesso ao servidor.\nReferenciado por: <@{}>", requester_id, selected_user_id))
        .colour(Colour::new(0x3498DB));

    let approve_btn = CreateButton::new(format!("approve_{}", requester_id))
        .label("Aprovar")
        .style(ButtonStyle::Success);
    let reject_btn = CreateButton::new(format!("reject_{}", requester_id))
        .label("Rejeitar")
        .style(ButtonStyle::Danger);
    let row = CreateActionRow::Buttons(vec![approve_btn, reject_btn]);

    if let Some(staff_channel_id) = &guild_config.staff_channel_id {
        if let Ok(channel_id) = staff_channel_id.parse::<u64>() {
            let _ = ChannelId::new(channel_id).send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row])).await;
        }
    }

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Sua solicitação foi enviada para aprovação da staff.")
            .ephemeral(true)
    )).await?;

    Ok(())
}

pub async fn handle_approval_action(ctx: &Context, interaction: &ComponentInteraction, approved: bool, guild_config: &Guild) -> Result<()> {
    let custom_id = &interaction.data.custom_id;
    let target_user_id = custom_id.split('_').nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if let Some(member) = &interaction.member {
        if !crate::permissions::is_admin(member) && !crate::permissions::is_staff(member, guild_config) {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Você não tem permissão para realizar esta ação.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    }

    if let Some(guild_id) = interaction.guild_id {
        if let Ok(mut member) = guild_id.member(&ctx.http, target_user_id).await {
            if approved {
                if let Some(role_id_str) = &guild_config.member_role_id {
                    if let Ok(role_id_num) = role_id_str.parse::<u64>() {
                        if let Err(e) = member.add_role(&ctx.http, RoleId::new(role_id_num)).await {
                            error!("Failed to add role to member: {}", e);
                        }
                    }
                }

                let dm_embed = CreateEmbed::new()
                    .title("Acesso Aprovado")
                    .description("Sua solicitação de acesso ao servidor foi aprovada!")
                    .colour(Colour::new(0x00FF00));
                let _ = member.user.direct_message(&ctx.http, CreateMessage::new().embed(dm_embed)).await;
            } else {
                let dm_embed = CreateEmbed::new()
                    .title("Acesso Rejeitado")
                    .description("Sua solicitação de acesso ao servidor foi rejeitada.")
                    .colour(Colour::new(0xFF0000));
                let _ = member.user.direct_message(&ctx.http, CreateMessage::new().embed(dm_embed)).await;
            }
        }
    }

    {
        let mut pending = PENDING_REQUESTS.lock().await;
        pending.remove(&target_user_id);
    }

    let updated_embed = CreateEmbed::new()
        .title("Solicitação de Acesso")
        .description(format!("<@{}> - {}", target_user_id, if approved { "Aprovado" } else { "Rejeitado" }))
        .colour(if approved { Colour::new(0x00FF00) } else { Colour::new(0xFF0000) });

    interaction.message.edit(&ctx.http, EditMessage::new().embed(updated_embed).components(vec![])).await?;

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(if approved { "Acesso aprovado com sucesso." } else { "Acesso rejeitado." })
            .ephemeral(true)
    )).await?;

    Ok(())
}
