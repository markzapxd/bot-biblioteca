use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("clearuser")
            .description("Delete messages from a specific user")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .add_option(CreateCommandOption::new(CommandOptionType::User, "user", "User whose messages to delete").required(true))
            .add_option(CreateCommandOption::new(CommandOptionType::Integer, "limit", "Max messages to scan (default 100)")),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let target = interaction.data.options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()).ok_or(crate::errors::BotError::Validation("User required".into()))?;
    let limit = interaction.data.options.iter().find(|o| o.name == "limit").and_then(|o| o.value.as_i64()).unwrap_or(100) as u8;

    interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true))).await?;

    let channel_id = interaction.channel_id;
    let messages = channel_id.messages(ctx, GetMessages::new().limit(limit.min(100))).await.unwrap_or_default();
    let to_delete: Vec<MessageId> = messages.iter().filter(|m| m.author.id == target).map(|m| m.id).collect();
    let count = to_delete.len();

    if count > 1 {
        let _ = channel_id.delete_messages(ctx, &to_delete).await;
    } else if count == 1 {
        let _ = channel_id.delete_message(ctx, to_delete[0]).await;
    }

    let embed = embeds::success("Messages Cleared", &format!("Deleted {} message(s) from <@{}>.", count, target));
    interaction.edit_response(ctx, EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
