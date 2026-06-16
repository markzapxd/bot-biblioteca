use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::repositories::guild_repo;
use crate::theme;

const MODULES: &[(&str, &str)] = &[
    ("antiraid", "Anti-Raid"),
    ("logs", "Logs"),
    ("tickets", "Tickets"),
    ("voice_tracking", "Voice Tracking"),
    ("member_verification", "Verificacao"),
];

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("modulos")
            .description("Manage active modules")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let user_id = interaction.user.id.get();
    permissions::require_admin(user_id, member)?;

    let guild_id_str = guild_id.to_string();
    let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
    let modules = config.get_modules();

    let embed = build_modules_embed(&modules);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "modules", embed).await;
    let row = build_modules_row(&modules);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

pub async fn handle_toggle(ctx: &Context, component: &ComponentInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = component.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild_id_str = guild_id.to_string();

    let module_name = component.data.custom_id.strip_prefix("modules_toggle_")
        .ok_or_else(|| crate::errors::BotError::Validation("Invalid custom id".into()))?;

    let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
    let mut modules = config.get_modules();

    match module_name {
        "antiraid" => modules.antiraid = !modules.antiraid,
        "logs" => modules.logs = !modules.logs,
        "tickets" => modules.tickets = !modules.tickets,
        "voice_tracking" => modules.voice_tracking = !modules.voice_tracking,
        "member_verification" => modules.member_verification = !modules.member_verification,
        _ => return Err(crate::errors::BotError::Validation("Unknown module".into())),
    }

    let modules_json = serde_json::to_value(&modules)?;
    guild_repo::update_modules(pool, &guild_id_str, modules_json).await?;

    let embed = build_modules_embed(&modules);
    let (embed, attachment) = crate::asset_manager::prepare_embed(&ctx, "modules", embed).await;
    let row = build_modules_row(&modules);

    let mut msg = CreateInteractionResponseMessage::new().embed(embed).components(vec![row]);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    component.create_response(&ctx, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

fn build_modules_embed(modules: &crate::models::guild::GuildModules) -> CreateEmbed {
    let mut desc = String::new();
    for (name, label) in MODULES {
        let enabled = match *name {
            "antiraid" => modules.antiraid,
            "logs" => modules.logs,
            "tickets" => modules.tickets,
            "voice_tracking" => modules.voice_tracking,
            "member_verification" => modules.member_verification,
            _ => false,
        };
        let status = if enabled { "Ativo" } else { "Inativo" };
        desc.push_str(&format!("**{}**: {}\n", label, status));
    }
    theme::info("Modulos", &desc.trim())
}

fn build_modules_row(modules: &crate::models::guild::GuildModules) -> CreateActionRow {
    let buttons: Vec<CreateButton> = MODULES.iter().map(|(name, label)| {
        let enabled = match *name {
            "antiraid" => modules.antiraid,
            "logs" => modules.logs,
            "tickets" => modules.tickets,
            "voice_tracking" => modules.voice_tracking,
            "member_verification" => modules.member_verification,
            _ => false,
        };
        CreateButton::new(format!("modules_toggle_{}", name))
            .label(*label)
            .style(if enabled { ButtonStyle::Success } else { ButtonStyle::Secondary })
    }).collect();
    CreateActionRow::Buttons(buttons)
}
