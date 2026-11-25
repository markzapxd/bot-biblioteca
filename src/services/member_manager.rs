use super::support_state::{self, Key};

use serenity::all::*;

use crate::errors::Result;
use crate::models::Guild;

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

    modal_submit.defer_ephemeral(&ctx.http).await?;
    let guild = modal_submit.guild_id.ok_or_else(|| crate::errors::BotError::Validation("Guild only".into()))?;
    let member = guild.member(&ctx.http, modal_submit.user.id).await?;
    let role: u64 = guild_config.member_role_id.as_ref().and_then(|v| v.parse().ok())
        .ok_or_else(|| crate::errors::BotError::Validation("Cargo de membro não configurado".into()))?;
    if member.roles.contains(&RoleId::new(role)) {
        modal_submit.edit_response(&ctx.http, EditInteractionResponse::new().content("Você já possui acesso.")).await?;
        return Ok(());
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

    let staff_channel_id = guild_config.staff_channel_id.as_ref()
        .ok_or_else(|| crate::errors::BotError::Validation("Canal de staff nao configurado neste servidor.".into()))?;
    let channel_id: u64 = staff_channel_id.parse()
        .map_err(|_| crate::errors::BotError::Validation("Canal de staff configurado e invalido.".into()))?;

    // Validate the configured destination belongs to the current guild before reserving.
    let destination = ChannelId::new(channel_id).to_channel(&ctx.http).await?;
    if destination.guild().map(|c| c.guild_id) != Some(guild) {
        return Err(crate::errors::BotError::Validation("Canal de outra guild".into()));
    }
    let store = support_state::store(ctx).await?;
    let key = Key::new(guild.get(), user_id);
    if !store.reserve_request(key).await? {
        modal_submit.edit_response(&ctx.http, EditInteractionResponse::new().content("Você já possui uma solicitação pendente (24h).")).await?;
        return Ok(());
    }
    let sent = match ChannelId::new(channel_id).send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row])).await {
        Ok(message) => message,
        Err(error) => {
            store.release_request(key).await?;
            modal_submit.edit_response(&ctx.http, EditInteractionResponse::new().content("Falha ao enviar solicitação. Tente novamente.")).await?;
            return Err(error.into());
        }
    };
    store.attach_request(key, channel_id, sent.id.get()).await?;
    modal_submit.edit_response(&ctx.http, EditInteractionResponse::new().content("Sua solicitação foi enviada para aprovação.")).await?;

    Ok(())
}

pub async fn handle_approval_action(ctx: &Context, interaction: &ComponentInteraction, approved: bool, guild_config: &Guild) -> Result<()> {
    interaction.defer_ephemeral(&ctx.http).await?;
    let guild = interaction.guild_id.ok_or_else(|| crate::errors::BotError::Unauthorized("Guild only".into()))?;
    if !support_state::staff(ctx, guild, interaction.user.id, guild_config).await? {
        return Err(crate::errors::BotError::Unauthorized("Apenas staff autorizado".into()));
    }
    let target: u64 = interaction.data.custom_id.split('_').nth(1).and_then(|v| v.parse().ok()).filter(|v| *v != 0)
        .ok_or_else(|| crate::errors::BotError::Validation("Usuário inválido".into()))?;
    let store = support_state::store(ctx).await?;
    let result = store.decide(Key::new(guild.get(), target), interaction.channel_id.get(), interaction.message.id.get(), || async {
        // Revalidate current target membership; failures preserve pending state and controls.
        let member = guild.member(&ctx.http, UserId::new(target)).await?;
        if approved {
            let role: u64 = guild_config.member_role_id.as_ref().and_then(|v| v.parse().ok()).filter(|v| *v != 0)
                .ok_or_else(|| crate::errors::BotError::Validation("Cargo não configurado".into()))?;
            member.add_role(&ctx.http, RoleId::new(role)).await?;
        }
        Ok(())
    }).await;
    match result {
        Err(error) => {
            interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Falha ao processar. Solicitação preservada; tente novamente.")).await?;
            return Err(error);
        }
        Ok(false) => {
            interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Solicitação expirada, inválida ou já processada.")).await?;
            return Ok(());
        }
        Ok(true) => {}
    }
    interaction.channel_id.edit_message(&ctx.http, interaction.message.id, EditMessage::new()
        .embed(CreateEmbed::new().title("Solicitação de Acesso").description(format!("<@{}> — {}", target, if approved { "Aprovado" } else { "Rejeitado" })))
        .components(vec![])).await?;
    interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Solicitação processada.")).await?;
    // Success notifications are unreachable on failed role assignment or stale requests.
    if let Ok(user) = UserId::new(target).to_user(&ctx.http).await {
        let _ = user.direct_message(&ctx.http, CreateMessage::new().content(if approved { "Seu acesso foi aprovado!" } else { "Sua solicitação foi rejeitada." })).await;
    }
    Ok(())
}
