use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;
use crate::repositories::guild_repo;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("modulos")
            .description("Manage active modules")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "enable", "Enable a module")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "module", "Module name").required(true)),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "disable", "Disable a module")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "module", "Module name").required(true)),
            )
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "list", "List all modules")),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let guild_id_str = guild_id.to_string();
    let sub = interaction.data.options.first().ok_or(crate::errors::BotError::Validation("Subcommand required".into()))?;

    match sub.name.as_str() {
        "enable" | "disable" => {
            let module_name = sub.options.iter().find(|o| o.name == "module").and_then(|o| o.value.as_str()).ok_or(crate::errors::BotError::Validation("Module name required".into()))?;
            let enable = sub.name == "enable";

            let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
            let mut modules = config.get_modules();
            match module_name {
                "antiraid" => modules.antiraid = enable,
                "logs" => modules.logs = enable,
                "tickets" => modules.tickets = enable,
                "voice_tracking" => modules.voice_tracking = enable,
                "member_verification" => modules.member_verification = enable,
                _ => {
                    let embed = embeds::error("Invalid Module", &format!("Unknown module: {}", module_name));
                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
                    return Ok(());
                }
            }
            let modules_json = serde_json::to_value(&modules)?;
            guild_repo::update_modules(pool, &guild_id_str, modules_json.clone()).await?;
            guild_repo::update_field(pool, &guild_id_str, "modules", &modules_json.to_string()).await.ok();

            let status = if enable { "enabled" } else { "disabled" };
            let embed = embeds::success("Module Updated", &format!("Module `{}` has been {}.", module_name, status));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        "list" => {
            let config = guild_cache.get(&guild_id_str).ok_or(crate::errors::BotError::NotFound("Guild config not found".into()))?;
            let modules = config.get_modules();
            let status = |v: bool| if v { "✅" } else { "❌" };
            let text = format!(
                "{} antiraid\n{} logs\n{} tickets\n{} voice_tracking\n{} member_verification",
                status(modules.antiraid), status(modules.logs), status(modules.tickets),
                status(modules.voice_tracking), status(modules.member_verification),
            );
            let embed = CreateEmbed::new().title("Active Modules").description(text).colour(Colour::new(0x3498DB));
            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
        }
        _ => {}
    }
    Ok(())
}
