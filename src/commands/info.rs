use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(CreateCommand::new("info").description("Server statistics"));
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(crate::errors::BotError::Validation("Guild only".into()))?;
    let guild = guild_id.to_partial_guild(ctx).await?;

    let channels = guild_id.channels(ctx).await.unwrap_or_default();
    let roles = guild_id.roles(ctx).await.unwrap_or_default();

    let embed = CreateEmbed::new()
        .title(&guild.name)
        .colour(Colour::new(0x2B2D31))
        .field("Members", guild.approximate_member_count.unwrap_or(0).to_string(), true)
        .field("Channels", channels.len().to_string(), true)
        .field("Roles", roles.len().to_string(), true)
        .field("Created", guild.id.created_at().format("%Y-%m-%d").to_string(), true)
        .field("Boost Level", (u8::from(guild.premium_tier)).to_string(), true)
        .thumbnail(guild.icon_url().unwrap_or_default());

    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed))).await?;
    Ok(())
}
