pub mod addticket;
pub mod admin;
pub mod botstats;
pub mod card;
pub mod claimticket;
pub mod clearuser;
pub mod closeticket;
pub mod help;
pub mod info;
pub mod lastcall;
pub mod lockdown;
pub mod lookup;
pub mod modulos;
pub mod configlogs;
pub mod names;
pub mod nuke;
pub mod owner;
pub mod privacy;
pub mod raidmode;
pub mod removeticket;
pub mod shutdown;
pub mod stats;
pub mod sync;
pub mod ticketpanel;
pub mod userinfo;
pub mod verification;
pub mod voicehistory;

use serenity::all::*;
use crate::errors::Result;
use crate::state::BotState;

pub async fn register_all(commands: &mut Vec<CreateCommand>) {
    addticket::register(commands);
    admin::register(commands);
    botstats::register(commands);
    card::register(commands);
    claimticket::register(commands);
    clearuser::register(commands);
    closeticket::register(commands);
    help::register(commands);
    info::register(commands);
    lastcall::register(commands);
    lockdown::register(commands);
    lookup::register(commands);
    modulos::register(commands);
    configlogs::register(commands);
    names::register(commands);
    nuke::register(commands);
    privacy::register(commands);
    raidmode::register(commands);
    removeticket::register(commands);
    shutdown::register(commands);
    stats::register(commands);
    sync::register(commands);
    ticketpanel::register(commands);
    userinfo::register(commands);
    verification::register(commands);
    voicehistory::register(commands);
}

pub async fn route(ctx: &Context, interaction: &CommandInteraction, state: &BotState) -> Result<()> {
    if let Some(guild_id) = interaction.guild_id {
        let guild_id_str = guild_id.to_string();
        if state.guild_cache.get(&guild_id_str).is_none() {
            let config = match crate::repositories::guild_repo::find_by_id(&state.pool, &guild_id_str).await? {
                Some(c) => c,
                None => {
                    crate::repositories::guild_repo::upsert(&state.pool, &guild_id_str).await?
                }
            };
            state.guild_cache.set(guild_id_str, config);
        }
    }

    match interaction.data.name.as_str() {
        "addticket" => addticket::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "admin" => admin::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "botstats" => botstats::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "card" => card::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "claimticket" => claimticket::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "clearuser" | "limpar" => clearuser::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "closeticket" => closeticket::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "help" | "ajuda" => help::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "info" => info::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "lastcall" | "ultimochamada" => lastcall::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "lockdown" => lockdown::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "lookup" => lookup::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "modulos" => modulos::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "configlogs" => configlogs::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "names" | "nomes" => names::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "nuke" => nuke::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "owner" => owner::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "privacy" | "privacidade" => privacy::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "raidmode" | "modoraid" => raidmode::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "removeticket" => removeticket::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "shutdown" => shutdown::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "stats" | "ranking" => stats::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "sync" => sync::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "ticketpanel" => ticketpanel::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "userinfo" | "ficha" => userinfo::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "verify" | "v" => verification::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        "voicehistory" | "historicovoz" => voicehistory::handle(ctx, interaction, &state.pool, &state.guild_cache).await,
        _ => Ok(()),
    }
}
