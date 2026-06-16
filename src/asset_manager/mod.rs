use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serenity::all::{Colour, Context, CreateAttachment, CreateEmbed};
use tracing::warn;

use crate::errors::Result;
use crate::state::BotStateKey;

static DEFAULT_ASSETS: &[(&str, &str)] = &[
    ("help", "help.png"),
    ("info", "info.png"),
    ("stats", "stats.png"),
    ("lastcall", "lastcall.png"),
    ("names", "names.png"),
    ("voicehistory", "voicehistory.png"),
    ("userinfo", "userinfo.png"),
    ("clearuser", "clearuser.png"),
    ("nuke", "nuke.png"),
    ("lockdown", "lockdown.png"),
    ("verification", "verification.png"),
    ("lookup", "lookup.png"),
    ("warnings", "warnings.png"),
    ("ticketpanel", "ticketpanel.png"),
    ("closeticket", "closeticket.png"),
    ("claimticket", "claimticket.png"),
    ("addticket", "addticket.png"),
    ("removeticket", "removeticket.png"),
    ("sync", "sync.png"),
    ("botstats", "botstats.png"),
    ("shutdown", "shutdown.png"),
    ("privacy", "privacy.png"),
    ("raidmode", "raidmode.png"),
    ("modules", "modules.png"),
    ("setup", "setup.png"),
    ("ticket", "ticket.png"),
    ("admin", "admin.png"),
    ("default", "default.png"),
];

#[derive(Debug, Clone)]
pub struct AssetManager {
    assets_dir: PathBuf,
    mapping: HashMap<String, String>,
    cache: HashMap<String, Vec<u8>>,
}

impl AssetManager {
    pub fn new() -> Self {
        let assets_dir = resolve_assets_dir();

        let mapping: HashMap<String, String> = DEFAULT_ASSETS
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();

        let mut cache: HashMap<String, Vec<u8>> = HashMap::new();
        for (key, filename) in DEFAULT_ASSETS {
            let path = assets_dir.join(filename);
            match std::fs::read(&path) {
                Ok(data) => {
                    cache.insert(key.to_string(), data);
                }
                Err(e) => {
                    warn!("Asset '{}' not found at {:?}: {}", key, path, e);
                }
            }
        }

        Self { assets_dir, mapping, cache }
    }

    pub fn get_path(&self, name: &str) -> Option<PathBuf> {
        self.mapping.get(name).map(|f| self.assets_dir.join(f))
    }

    pub fn get_attachment_url(&self, name: &str) -> Option<String> {
        self.mapping.get(name).map(|f| format!("attachment://{}", f))
    }

    pub fn get_filename(&self, name: &str) -> Option<&str> {
        self.mapping.get(name).map(|s| s.as_str())
    }

    pub fn has_asset(&self, name: &str) -> bool {
        self.cache.contains_key(name)
    }

    /// Build an embed with a thumbnail attachment.
    ///
    /// Returns `(embed_with_thumbnail, Some(CreateAttachment))` if the asset is cached,
    /// or `(embed_unchanged, None)` if not.
    pub fn embed_with(&self, name: &str, embed: CreateEmbed) -> (CreateEmbed, Option<CreateAttachment>) {
        let url = match self.get_attachment_url(name) {
            Some(u) => u,
            None => return (embed, None),
        };
        let data = match self.cache.get(name) {
            Some(d) => d.clone(),
            None => return (embed, None),
        };
        let filename = match self.mapping.get(name) {
            Some(f) => f.clone(),
            None => return (embed, None),
        };
        let embed = embed.thumbnail(url);
        let attachment = CreateAttachment::bytes(data, filename);
        (embed, Some(attachment))
    }

    /// Build an embed with a large image attachment.
    pub fn embed_with_large(&self, name: &str, embed: CreateEmbed) -> (CreateEmbed, Option<CreateAttachment>) {
        let url = match self.get_attachment_url(name) {
            Some(u) => u,
            None => return (embed, None),
        };
        let data = match self.cache.get(name) {
            Some(d) => d.clone(),
            None => return (embed, None),
        };
        let filename = match self.mapping.get(name) {
            Some(f) => f.clone(),
            None => return (embed, None),
        };
        let embed = embed.image(url);
        let attachment = CreateAttachment::bytes(data, filename);
        (embed, Some(attachment))
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn prepare_embed(
    ctx: &Context,
    name: &str,
    embed: CreateEmbed,
) -> (CreateEmbed, Option<CreateAttachment>) {
    if let Some(state) = ctx.data.read().await.get::<BotStateKey>() {
        state.asset_manager.embed_with(name, embed)
    } else {
        (embed, None)
    }
}

pub async fn prepare_embed_large(
    ctx: &Context,
    name: &str,
    embed: CreateEmbed,
) -> (CreateEmbed, Option<CreateAttachment>) {
    if let Some(state) = ctx.data.read().await.get::<BotStateKey>() {
        state.asset_manager.embed_with_large(name, embed)
    } else {
        (embed, None)
    }
}

fn resolve_assets_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ASSETS_DIR") {
        return PathBuf::from(dir);
    }
    let base = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd
    } else {
        PathBuf::from(".")
    };
    base.join("assets").join("images")
}
