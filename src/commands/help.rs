use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(CreateCommand::new("help").description("List all available commands"));
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    let embed = CreateEmbed::new()
        .title("Bibliotecário — Commands")
        .colour(Colour::new(0x3498DB))
        .field("Moderation", "`/admin ban|kick|mute|unmute|warn`", false)
        .field("Messages", "`/clearuser` `/nuke`", false)
        .field("Voice", "`/lastcall` `/stats` `/voicehistory`", false)
        .field("Users", "`/userinfo` `/names` `/privacy`", false)
        .field("Server", "`/info` `/setup` `/setup-frin` `/modulos`", false)
        .field("Tickets", "`/ticket`", false)
        .field("Verification", "`/v`", false)
        .field("Security", "`/raidmode` `/lockdown`", false)
        .field("Owner", "`/owner userinfo|names|voicehistory|resetuser|setprivate|botinfo|clearsessions|synccommands`", false)
        .field("Other", "`/help`", false);

    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(embed).ephemeral(true))).await?;
    Ok(())
}
