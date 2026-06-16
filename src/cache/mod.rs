use crate::models::Guild;
use dashmap::DashMap;

pub struct GuildCache {
    inner: DashMap<String, Guild>,
}

impl GuildCache {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub fn get(&self, guild_id: &str) -> Option<Guild> {
        self.inner.get(guild_id).map(|r| r.clone())
    }

    pub fn set(&self, guild_id: String, config: Guild) {
        self.inner.insert(guild_id, config);
    }

    pub fn remove(&self, guild_id: &str) {
        self.inner.remove(guild_id);
    }

    pub fn load_all(&self, guilds: Vec<Guild>) {
        for g in guilds {
            self.inner.insert(g.guild_id.clone(), g);
        }
    }
}
