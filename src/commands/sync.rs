use serenity::all::*;
use sqlx::PgPool;
use crate::cache::GuildCache;
use crate::errors::Result;
use crate::permissions;
use crate::theme;

pub fn register(commands: &mut Vec<CreateCommand>) {
    commands.push(
        CreateCommand::new("sync")
            .description("Re-registrar comandos em todos os servidores")
            .default_member_permissions(Permissions::ADMINISTRATOR),
    );
}

pub async fn handle(ctx: &Context, interaction: &CommandInteraction, _pool: &PgPool, _guild_cache: &GuildCache) -> Result<()> {
    permissions::require_owner(interaction.user.id.get())?;

    interaction.defer(ctx).await?;

    let _ = Command::set_global_commands(ctx, vec![]).await;

    let mut cmds = Vec::new();
    crate::commands::register_all(&mut cmds).await;

    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        let _ = guild_id.set_commands(ctx, cmds.clone()).await;
    }

    let embed = theme::success("Comandos Sincronizados", &format!("Registrados {} comandos em todos os servidores.", cmds.len()));
    interaction.edit_response(ctx, serenity::all::EditInteractionResponse::new().embed(embed)).await?;
    Ok(())
}
