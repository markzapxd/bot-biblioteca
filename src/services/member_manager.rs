use std::collections::HashSet;
use std::sync::LazyLock;

use serenity::all::*;
use tracing::error;

use crate::errors::Result;
use crate::models::Guild;

static PENDING_REQUESTS: LazyLock<tokio::sync::Mutex<HashSet<u64>>> = LazyLock::new(|| tokio::sync::Mutex::new(HashSet::new()));

pub async fn handle_access_request(ctx: &Context, interaction: &ComponentInteraction, guild_config: &Guild) -> Result<()> {
    let user_id = interaction.user.id.get();

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

    let input = CreateInputText::new(InputTextStyle::Short, "Quem te chamou para o servidor?", "referral_input")
        .placeholder("Digite o nome ou apelido da pessoa")
        .max_length(100);

    let modal = CreateModal::new("referral_modal", "Verificação de Acesso")
        .components(vec![CreateActionRow::InputText(input)]);

    interaction.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;

    Ok(())
}

pub async fn handle_referral_modal_submit(ctx: &Context, modal_submit: &ModalInteraction, guild_config: &Guild) -> Result<()> {
    let user_id = modal_submit.user.id.get();

    {
        let mut pending = PENDING_REQUESTS.lock().await;
        if pending.contains(&user_id) {
            modal_submit.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Você já possui uma solicitação de acesso pendente.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
        pending.insert(user_id);
    }

    let referral_text = modal_submit.data.components.first()
        .and_then(|row| row.components.first())
        .and_then(|comp| {
            if let ActionRowComponent::InputText(input) = comp {
                input.value.clone()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Não informado".to_string());

    let avatar_url = modal_submit.user.face();

    let embed = CreateEmbed::new()
        .title("Solicitação de Acesso")
        .description(format!("<@{}> solicitou acesso ao servidor.", user_id))
        .field("Quem chamou", format!("**`{}`**", referral_text), false)
        .field("ID do usuário", format!("`{}`", user_id), true)
        .thumbnail(avatar_url)
        .colour(Colour::new(0x2B2D31));

    let approve_btn = CreateButton::new(format!("approve_{}", user_id))
        .label("Aprovar")
        .style(ButtonStyle::Secondary);
    let reject_btn = CreateButton::new(format!("reject_{}", user_id))
        .label("Rejeitar")
        .style(ButtonStyle::Secondary);
    let row = CreateActionRow::Buttons(vec![approve_btn, reject_btn]);

    if let Some(staff_channel_id) = &guild_config.staff_channel_id {
        if let Ok(channel_id) = staff_channel_id.parse::<u64>() {
            let _ = ChannelId::new(channel_id).send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row])).await;
        }
    }

    modal_submit.create_response(&ctx.http, CreateInteractionResponse::Message(
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
        if !crate::permissions::is_admin(member, guild_config) && !crate::permissions::is_staff(member, guild_config) {
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Você não tem permissão para realizar esta ação.")
                    .ephemeral(true)
            )).await?;
            return Ok(());
        }
    }

    let thumbnail_url = interaction.message.embeds.first()
        .and_then(|e| e.thumbnail.as_ref())
        .map(|t| t.url.clone());

    let mut updated_embed = CreateEmbed::new()
        .title("Solicitação de Acesso")
        .description(format!("<@{}> — {}", target_user_id, if approved { "Aprovado" } else { "Rejeitado" }))
        .colour(Colour::new(0x2B2D31));

    if let Some(url) = thumbnail_url {
        updated_embed = updated_embed.thumbnail(url);
    }

    interaction.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(updated_embed)
            .components(vec![])
    )).await?;

    let ctx_clone = ctx.clone();
    let interaction_clone = interaction.clone();
    let guild_config_clone = guild_config.clone();

    tokio::spawn(async move {
        if let Some(guild_id) = interaction_clone.guild_id {
            if let Ok(member) = guild_id.member(&ctx_clone.http, target_user_id).await {
                if approved {
                    if let Some(role_id_str) = &guild_config_clone.member_role_id {
                        if let Ok(role_id_num) = role_id_str.parse::<u64>() {
                            if let Err(e) = member.add_role(&ctx_clone.http, RoleId::new(role_id_num)).await {
                                error!("Failed to add role to member: {}", e);
                            }
                        }
                    }

                    let dm_embed = CreateEmbed::new()
                        .title("Acesso Aprovado")
                        .description("Sua solicitação de acesso ao servidor foi aprovada!")
                        .colour(Colour::new(0x2B2D31));
                    let (dm_embed, attachment) = crate::asset_manager::prepare_embed_large(&ctx_clone, "approved", dm_embed).await;
                    let mut msg = CreateMessage::new().embed(dm_embed);
                    if let Some(file) = attachment {
                        msg = msg.add_file(file);
                    }
                    let _ = member.user.direct_message(&ctx_clone.http, msg).await;
                } else {
                    let dm_embed = CreateEmbed::new()
                        .title("Acesso Rejeitado")
                        .description("Sua solicitação de acesso ao servidor foi rejeitada.")
                        .colour(Colour::new(0x2B2D31));
                    let (dm_embed, attachment) = crate::asset_manager::prepare_embed_large(&ctx_clone, "rejected", dm_embed).await;
                    let mut msg = CreateMessage::new().embed(dm_embed);
                    if let Some(file) = attachment {
                        msg = msg.add_file(file);
                    }
                    let _ = member.user.direct_message(&ctx_clone.http, msg).await;
                }
            }
        }

        {
            let mut pending = PENDING_REQUESTS.lock().await;
            pending.remove(&target_user_id);
        }
    });

    Ok(())
}
