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

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let embed = CreateEmbed::new()
        .title("Owner Panel")
        .description("Controles exclusivos do dono.")
        .colour(Colour::new(0x2B2D31));

    let finally_btn = CreateButton::new("owner_finally")
        .label("Finally")
        .style(ButtonStyle::Danger);
    let clear_btn = CreateButton::new("owner_clear")
        .label("Clear")
        .style(ButtonStyle::Secondary);

    let row = CreateActionRow::Buttons(vec![finally_btn, clear_btn]);

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![row])
        .ephemeral(true);

    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
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

pub async fn handle_clear(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let guild_id = interaction.guild_id.ok_or(BotError::Validation("Guild only".into()))?;

    interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await?;

    let channels = guild_id.channels(&ctx.http).await?;

    for (_, guild_channel) in channels {
        if guild_channel.kind == ChannelType::Text {
            let mut messages = guild_channel.messages(&ctx.http, GetMessages::new().limit(100)).await?;
            while !messages.is_empty() {
                let ids: Vec<MessageId> = messages.iter().map(|m| m.id).collect();
                let _ = guild_channel.delete_messages(&ctx.http, &ids).await;
                messages = guild_channel.messages(&ctx.http, GetMessages::new().limit(100)).await?;
            }
        }
    }

    interaction.edit_response(&ctx.http, EditInteractionResponse::new()
        .content("Todos os canais de texto foram limpos.")
        .embeds(vec![])
        .components(vec![])
    ).await?;

    Ok(())
}
