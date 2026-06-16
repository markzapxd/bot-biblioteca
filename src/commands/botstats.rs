use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::utils::time;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("botstats")
            .description("Estatisticas do bot")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    let guilds = ctx.cache.guild_count();
    let users = ctx.cache.user_count();
    let uptime = if let Some(state) = ctx.data.read().await.get::<crate::state::BotStateKey>() {
        chrono::Utc::now() - state.start_time
    } else {
        chrono::Duration::zero()
    };

    let embed = CreateEmbed::new()
        .title("Bot Stats")
        .colour(crate::theme::Theme::SUCCESS)
        .field("Servidores", guilds.to_string(), true)
        .field("Usuarios em Cache", users.to_string(), true)
        .field("Uptime", time::format_duration(uptime.num_milliseconds()), true);
    let (embed, attachment) = crate::asset_manager::prepare_embed(ctx, "botstats", embed).await;

    let mut msg = CreateInteractionResponseMessage::new().embed(embed);
    if let Some(file) = attachment {
        msg = msg.add_file(file);
    }
    interaction.create_response(ctx, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
