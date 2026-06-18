use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::{BotError, Result};
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("owner")
            .description("Comandos exclusivos do dono")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

fn owner_panel_buttons() -> Vec<CreateActionRow> {
    let finally_btn = CreateButton::new("owner_finally")
        .label("Finally")
        .style(ButtonStyle::Danger);
    let clear_btn = CreateButton::new("owner_clear")
        .label("Clear")
        .style(ButtonStyle::Secondary);
    let setrole_btn = CreateButton::new("owner_setrole")
        .label("Set Role")
        .style(ButtonStyle::Primary);
    vec![CreateActionRow::Buttons(vec![finally_btn, clear_btn, setrole_btn])]
}

pub async fn handle_prefix(ctx: &Context, msg: &Message) -> Result<()> {
    permissions::require_owner(msg.author.id.get())?;

    let embed = CreateEmbed::new()
        .title("Owner Panel")
        .description("Controles exclusivos do dono.")
        .colour(Colour::new(0x2B2D31));

    msg.channel_id.send_message(ctx, CreateMessage::new()
        .embed(embed)
        .components(owner_panel_buttons())
    ).await?;

    Ok(())
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let embed = CreateEmbed::new()
        .title("Owner Panel")
        .description("Controles exclusivos do dono.")
        .colour(Colour::new(0x2B2D31));

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(owner_panel_buttons())
        .ephemeral(true);

    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_setrole_button(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let user_input = CreateInputText::new(InputTextStyle::Short, "User ID", "owner_setrole_user")
        .placeholder("ID do usuario")
        .required(true);
    let role_input = CreateInputText::new(InputTextStyle::Short, "Role ID", "owner_setrole_role")
        .placeholder("ID do cargo")
        .required(true);

    let modal = CreateModal::new("owner_setrole", "Atribuir Cargo")
        .components(vec![
            CreateActionRow::InputText(user_input),
            CreateActionRow::InputText(role_input),
        ]);

    component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
    Ok(())
}

fn extract_modal_row(rows: &[ActionRow], custom_id: &str) -> Option<String> {
    rows.iter().flat_map(|row| row.components.clone()).find_map(|comp| {
        if let ActionRowComponent::InputText(input) = &comp {
            if input.custom_id == custom_id {
                return input.value.clone();
            }
        }
        None
    })
}

pub async fn handle_setrole_modal(ctx: &Context, modal: &ModalInteraction) -> Result<()> {
    let user_id_str = extract_modal_row(&modal.data.components, "owner_setrole_user")
        .ok_or_else(|| BotError::Validation("User ID nao fornecido".into()))?;
    let role_id_str = extract_modal_row(&modal.data.components, "owner_setrole_role")
        .ok_or_else(|| BotError::Validation("Role ID nao fornecido".into()))?;

    let user_id: u64 = user_id_str.trim().parse()
        .map_err(|_| BotError::Validation("User ID invalido".into()))?;
    let role_id: u64 = role_id_str.trim().parse()
        .map_err(|_| BotError::Validation("Role ID invalido".into()))?;

    let guild_id = modal.guild_id
        .ok_or_else(|| BotError::Validation("Guild only".into()))?;

    let role = guild_id.roles(&ctx.http).await?
        .get(&RoleId::new(role_id))
        .cloned()
        .ok_or_else(|| BotError::NotFound("Cargo nao encontrado no servidor".into()))?;

    guild_id.member(&ctx.http, user_id).await
        .map_err(|_| BotError::NotFound("Usuario nao encontrado no servidor".into()))?
        .add_role(&ctx.http, RoleId::new(role_id)).await
        .map_err(|e| BotError::Internal(format!("Falha ao atribuir cargo: {}", e)))?;

    modal.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("Cargo `{}` atribuido a <@{}>", role.name, user_id))
            .ephemeral(true)
    )).await?;

    Ok(())
}

pub async fn handle_finally(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;

    interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let channels = guild_id.channels(&ctx.http).await?;

    for (_, channel) in channels {
        let _ = channel.id.delete(&ctx.http).await;
    }

    let text_category = guild_id.create_channel(&ctx.http, CreateChannel::new(".")
        .kind(ChannelType::Category)).await?;
    let voice_category = guild_id.create_channel(&ctx.http, CreateChannel::new(".")
        .kind(ChannelType::Category)).await?;

    guild_id.create_channel(&ctx.http, CreateChannel::new("msg")
        .kind(ChannelType::Text)
        .category(text_category.id)).await?;
    guild_id.create_channel(&ctx.http, CreateChannel::new("cmd")
        .kind(ChannelType::Text)
        .category(text_category.id)).await?;
    guild_id.create_channel(&ctx.http, CreateChannel::new("vc 1")
        .kind(ChannelType::Voice)
        .category(voice_category.id)).await?;
    guild_id.create_channel(&ctx.http, CreateChannel::new("vc 2")
        .kind(ChannelType::Voice)
        .category(voice_category.id)).await?;

    interaction.edit_response(&ctx.http, EditInteractionResponse::new()
        .content("Servidor reconstruido.")
        .embeds(vec![])
        .components(vec![])
    ).await?;

    Ok(())
}

pub async fn handle_clear_button(ctx: &Context, component: &ComponentInteraction) -> Result<()> {
    let channel_input = CreateInputText::new(InputTextStyle::Short, "Channel ID (opcional)", "owner_clear_channel")
        .placeholder("Deixe vazio para o canal atual")
        .required(false);
    let count_input = CreateInputText::new(InputTextStyle::Short, "Quantidade", "owner_clear_count")
        .placeholder("Numero de mensagens para apagar")
        .required(true);

    let modal = CreateModal::new("owner_clear", "Limpar Mensagens")
        .components(vec![
            CreateActionRow::InputText(channel_input),
            CreateActionRow::InputText(count_input),
        ]);

    component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
    Ok(())
}

pub async fn handle_clear_modal(ctx: &Context, modal: &ModalInteraction) -> Result<()> {
    permissions::require_owner(modal.user.id.get())?;

    let guild_id = modal.guild_id
        .ok_or_else(|| BotError::Validation("Guild only".into()))?;

    let channel_id_str = extract_modal_row(&modal.data.components, "owner_clear_channel")
        .and_then(|v| if v.trim().is_empty() { None } else { Some(v.trim().to_string()) });

    let target_channel = if let Some(cid) = channel_id_str {
        let id: u64 = cid.parse()
            .map_err(|_| BotError::Validation("Channel ID invalido".into()))?;
        ChannelId::new(id)
    } else {
        modal.channel_id
    };

    let count_str = extract_modal_row(&modal.data.components, "owner_clear_count")
        .ok_or_else(|| BotError::Validation("Quantidade nao fornecida".into()))?;
    let count: usize = count_str.trim().parse()
        .map_err(|_| BotError::Validation("Quantidade invalida".into()))?;

    modal.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let mut remaining = count.min(1000);
    while remaining > 0 {
        let batch = remaining.min(100);
        match target_channel.messages(&ctx.http, GetMessages::new().limit(batch as u8)).await {
            Ok(messages) => {
                if messages.is_empty() {
                    break;
                }
                let ids: Vec<MessageId> = messages.iter().map(|m| m.id).collect();
                let _ = target_channel.delete_messages(&ctx.http, &ids).await;
                remaining = remaining.saturating_sub(ids.len());
            }
            Err(e) => {
                modal.edit_response(&ctx.http, EditInteractionResponse::new()
                    .content(format!("Erro ao limpar: {}", e))
                ).await?;
                return Ok(());
            }
        }
    }

    modal.edit_response(&ctx.http, EditInteractionResponse::new()
        .content(format!("Limpo <#{}> ({} mensagens).", target_channel, count.min(1000)))
    ).await?;

    Ok(())
}
