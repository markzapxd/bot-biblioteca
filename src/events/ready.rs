use serenity::all::{Command, Context, Ready};

pub async fn handle(ctx: Context, ready: Ready) {
    tracing::info!("{} is connected!", ready.user.name);

    let _ = Command::set_global_commands(&ctx, vec![]).await;

    let mut cmds = Vec::new();
    crate::commands::register_all(&mut cmds).await;

    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        if let Err(e) = guild_id.set_commands(&ctx, cmds.clone()).await {
            tracing::error!("Failed to register guild commands for {}: {}", guild_id, e);
        } else {
            tracing::info!("Registered guild commands for {}", guild_id);
        }
    }
}
