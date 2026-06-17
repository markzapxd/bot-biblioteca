use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::repositories::user_repo;
use crate::services::history_manager;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("names")
            .description("User history — names and nicknames")
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "Target user").required(true)),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let user_id_str = target.to_string();

    let user = user_repo::find_or_create(pool, &user_id_str).await?;

    if user.is_private_mode() {
        let requester = interaction.user.id;
        let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
        let guild_config = _guild_cache.get(&guild_id.to_string())
            .ok_or_else(|| crate::errors::BotError::NotFound("Guild config not found".into()))?;
        let is_staff = interaction.member.as_ref().map(|m| permissions::is_admin(m, &guild_config)).unwrap_or(false);
        if requester != target && !is_staff && !permissions::is_owner(requester.get()) {
            let embed = embeds::warning("Private", "This user has privacy mode enabled.");
            let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "names", embed).await;
            let mut msg = CreateInteractionResponseMessage::new().embed(embed).ephemeral(true);
            if let Some(file) = attachment {
                msg = msg.add_file(file);
            }
            interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
            return Ok(());
        }
    }

    let username_history = user.get_username_history();
    let (embed, rows) = history_manager::build_names_page(target.get(), &username_history);

    let msg = CreateInteractionResponseMessage::new().embed(embed).components(rows);

    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

use crate::permissions;
