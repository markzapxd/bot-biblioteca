pub mod admin;
pub mod clearuser;
pub mod help;
pub mod info;
pub mod lastcall;
pub mod lockdown;
pub mod modulos;
pub mod names;
pub mod nuke;
pub mod owner;
pub mod privacy;
pub mod raidmode;
pub mod setup;
pub mod setup_frin;
pub mod stats;
pub mod ticket;
pub mod userinfo;
pub mod verification;
pub mod voicehistory;

use serenity::all::*;
use crate::errors::Result;
use crate::state::BotState;

pub async fn register_all(commands: &mut Vec<CreateCommand>) {
    admin::register(commands);
    clearuser::register(commands);
    help::register(commands);
    info::register(commands);
    lastcall::register(commands);
    lockdown::register(commands);
    modulos::register(commands);
    names::register(commands);
    nuke::register(commands);
    owner::register(commands);
    privacy::register(commands);
    raidmode::register(commands);
    setup::register(commands);
    setup_frin::register(commands);
    stats::register(commands);
    ticket::register(commands);
    userinfo::register(commands);
    verification::register(commands);
    voicehistory::register(commands);
}

pub async fn route(ctx: &Context, interaction: &CommandInteraction, state: &BotState) -> Result<()> {
    match interaction.data.name.as_str() {
        "admin" => admin::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "clearuser" => clearuser::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "help" => help::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "info" => info::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "lastcall" => lastcall::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "lockdown" => lockdown::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "modulos" => modulos::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "names" => names::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "nuke" => nuke::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "owner" => owner::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "privacy" => privacy::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "raidmode" => raidmode::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "setup" => setup::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "setup-frin" => setup_frin::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "stats" => stats::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "ticket" => ticket::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "userinfo" => userinfo::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "v" => verification::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "voicehistory" => voicehistory::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        _ => Ok(()),
    }
}
