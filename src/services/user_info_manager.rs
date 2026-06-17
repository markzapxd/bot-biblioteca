use serenity::all::*;
use sqlx::PgPool;
use tracing::error;

use crate::errors::Result;
use crate::models::Guild;

pub struct UserInfoResult {
    pub embed: Option<CreateEmbed>,
    pub action_row: Option<CreateActionRow>,
    pub restricted: bool,
    pub content: Option<String>,
}

pub async fn build_user_info(
    ctx: &Context,
    target_user: &User,
    guild: &Guild,
    interaction_user_id: u64,
    is_staff: bool,
    pool: &PgPool,
) -> Result<UserInfoResult> {
    let user = crate::repositories::user_repo::find_or_create(pool, &target_user.id.to_string()).await?;

    let is_self = target_user.id.get() == interaction_user_id;
    let is_private = user.is_private_mode();

    if is_private && !is_staff && !is_self {
        return Ok(UserInfoResult {
            embed: None,
            action_row: None,
            restricted: true,
            content: Some("Este perfil é privado.".to_string()),
        });
    }

    let avatar_history = user.get_avatar_history();
    let has_history = !avatar_history.is_empty();

    let account_created = target_user.created_at().with_timezone(&chrono::Utc);
    let guild_join_date = if let Ok(member) = guild.guild_id.parse::<u64>() {
        let guild_id = GuildId::new(member);
        if let Ok(member_obj) = guild_id.member(&ctx.http, target_user.id).await {
            member_obj.joined_at.map(|t| t.with_timezone(&chrono::Utc))
        } else {
            None
        }
    } else {
        None
    };

    let total_voice = user.total_voice_time.unwrap_or(0);
    let voice_formatted = crate::utils::time::format_duration(total_voice);

    let mut embed = CreateEmbed::new()
        .title(format!("Ficha de {}", target_user.name))
        .thumbnail(target_user.face())
        .field("ID", target_user.id.to_string(), true)
        .field("Conta Criada", account_created.format("%d/%m/%Y %H:%M").to_string(), true)
        .field("Privado", if is_private { "Sim" } else { "Não" }, true)
        .field("Tempo em Voz", voice_formatted, true)
        .colour(Colour::new(0x2B2D31));

    if let Some(joined) = guild_join_date {
        embed = embed.field("Entrou no Servidor", joined.format("%d/%m/%Y %H:%M").to_string(), true);
    }

    let avatar_btn = CreateButton::new(format!("avatar_{}_0", target_user.id.get()))
        .label("Histórico de Avatares")
        .style(ButtonStyle::Secondary)
        .disabled(!has_history);
    let row = CreateActionRow::Buttons(vec![avatar_btn]);

    Ok(UserInfoResult {
        embed: Some(embed),
        action_row: Some(row),
        restricted: false,
        content: None,
    })
}

pub async fn handle_user_info_back(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
    let custom_id = &interaction.data.custom_id;
    let target_user_id = custom_id.split('_').nth(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let target_user = match UserId::new(target_user_id).to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to fetch user for user info back: {}", e);
            interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Erro ao buscar informações do usuário.")
                    .ephemeral(true)
            )).await?;
            return Err(crate::errors::BotError::Discord(e));
        }
    };

    let guild_id = interaction.guild_id.map(|g| g.get()).unwrap_or(0);
    let guild_config = if guild_id > 0 {
        crate::repositories::guild_repo::find_by_id(pool, &guild_id.to_string()).await?.unwrap_or_else(|| crate::models::Guild {
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
        })
    } else {
        crate::models::Guild {
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
    };

    let is_staff = if let Some(member) = &interaction.member {
        crate::permissions::is_staff(member, &guild_config)
    } else {
        false
    };

    let result = build_user_info(ctx, &target_user, &guild_config, interaction.user.id.get(), is_staff, pool).await?;

    if let Some(embed) = result.embed {
        let mut edit = EditInteractionResponse::new().embed(embed);
        if let Some(row) = result.action_row {
            edit = edit.components(vec![row]);
        }
        interaction.edit_response(&ctx.http, edit).await?;
    }

    Ok(())
}
