use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::embeds;
use crate::permissions;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("nuke")
            .description("Clone and recreate the current channel")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let member = interaction.member.as_ref().ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    permissions::require_admin(member)?;

    let channel_id = interaction.channel_id;

    interaction.create_response(ctx, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

    let channel = channel_id.to_channel(ctx).await?;
    if let Channel::Guild(guild_channel) = channel {
        let new_channel = guild_channel.clone();
        let _ = channel_id.delete(ctx).await;

        if let Some(guild_id) = new_channel.guild_id {
            let builder = CreateChannel::new(&new_channel.name).kind(new_channel.kind);
            if let Ok(created) = guild_id.create_channel(ctx, builder).await {
                let embed = embeds::success("Channel Nuked", "Channel has been cloned and recreated.");
                let _ = created.send_message(ctx, CreateMessage::new().embed(embed)).await;
            }
        }
    }

    Ok(())
}
