use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("lookup")
            .description("Ficha completa de usuario com acoes de moderacao")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "user", "Usuario a consultar")
                    .required(true),
            ),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;

    let target = interaction.data.options.first()
        .and_then(|o| o.value.as_user_id())
        .ok_or(crate::errors::BotError::Validation("User required".into()))?;

    let target_member = guild_id.member(ctx, target).await?;
    let target_user = target.to_user(ctx).await?;

    let join_date = target_member.joined_at
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Desconhecido".into());

    let roles: Vec<String> = target_member.roles.iter()
        .take(20)
        .map(|r| format!("<@&{}>", r.get()))
        .collect();
    let roles_str = if roles.is_empty() { "Nenhum".into() } else { roles.join(" ") };

    let embed = CreateEmbed::new()
        .title(format!("Lookup — {}", target_user.name))
        .thumbnail(target_user.avatar_url().unwrap_or_default())
        .colour(theme::Theme::SUCCESS)
        .field("Usuario", format!("<@{}>", target.get()), true)
        .field("ID", target.get().to_string(), true)
        .field("Entrou em", join_date, true)
        .field("Cargos", roles_str, false);

    let user_id_str = target.get().to_string();
    let buttons = vec![
        CreateButton::new(format!("lookup_ban_{}", user_id_str))
            .label("Banir")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("lookup_kick_{}", user_id_str))
            .label("Expulsar")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("lookup_mute_{}", user_id_str))
            .label("Silenciar")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("lookup_warn_{}", user_id_str))
            .label("Advertir")
            .style(ButtonStyle::Secondary),
    ];
    let row = CreateActionRow::Buttons(buttons);

    interaction.create_response(ctx, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().embed(embed).components(vec![row])
    )).await?;
    Ok(())
}

pub async fn handle_ban(ctx: &Context, component: &ComponentInteraction, target: UserId) -> Result<()> {
    let member = component.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let caller_id = component.user.id.get();
    permissions::require_moderator(caller_id, member)?;

    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    if let Ok(m) = guild_id.member(ctx, target).await {
        let _ = m.ban_with_reason(ctx, 0, "Banned via lookup").await;
    } else {
        let _ = guild_id.ban_with_reason(ctx, target, 0, "Banned via lookup").await;
    }
    let embed = theme::success("Usuario Banido", &format!("<@{}> foi banido.", target.get()));
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().embed(embed).components(vec![])
    )).await?;
    Ok(())
}

pub async fn handle_kick(ctx: &Context, component: &ComponentInteraction, target: UserId) -> Result<()> {
    let member = component.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let caller_id = component.user.id.get();
    permissions::require_moderator(caller_id, member)?;

    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let m = guild_id.member(ctx, target).await?;
    m.kick_with_reason(ctx, "Kicked via lookup").await?;
    let embed = theme::success("Usuario Expulso", &format!("<@{}> foi expulso.", target.get()));
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().embed(embed).components(vec![])
    )).await?;
    Ok(())
}

pub async fn handle_mute(ctx: &Context, component: &ComponentInteraction, target: UserId) -> Result<()> {
    let member = component.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let caller_id = component.user.id.get();
    permissions::require_moderator(caller_id, member)?;

    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let mut m = guild_id.member(ctx, target).await?;
    let until_ts = (chrono::Utc::now() + chrono::Duration::minutes(10)).timestamp();
    let since_epoch = serenity::model::Timestamp::from_unix_timestamp(until_ts)
        .map_err(|e| crate::errors::BotError::Internal(e.to_string()))?;
    m.disable_communication_until_datetime(&ctx.http, since_epoch).await?;
    let embed = theme::success("Usuario Silenciado", &format!("<@{}> silenciado por 10 minutos.", target.get()));
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().embed(embed).components(vec![])
    )).await?;
    Ok(())
}

pub async fn handle_warn(ctx: &Context, component: &ComponentInteraction, target: UserId) -> Result<()> {
    let member = component.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let caller_id = component.user.id.get();
    permissions::require_moderator(caller_id, member)?;

    if let Ok(user) = target.to_user(ctx).await {
        if let Ok(dm) = user.create_dm_channel(ctx).await {
            let warn_embed = CreateEmbed::new()
                .title("Advertencia")
                .description("Voce recebeu uma advertencia da staff.")
                .colour(Colour::new(0x2B2D31));
            let _ = dm.send_message(ctx, CreateMessage::new().embed(warn_embed)).await;
        }
    }
    let embed = theme::success("Advertencia Enviada", &format!("<@{}> foi advertido.", target.get()));
    component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new().embed(embed).components(vec![])
    )).await?;
    Ok(())
}
