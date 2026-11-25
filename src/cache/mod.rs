use crate::models::Guild;
use dashmap::DashMap;

#[derive(Clone)]
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
            // Historical rows must not admit membership or resurrect removals.
            if let Some(mut current) = self.inner.get_mut(&g.guild_id) {
                *current = g;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_rows_do_not_admit_membership() {
        let cache = GuildCache::new();
        let guild: Guild = serde_json::from_value(serde_json::json!({
            "guild_id": "1", "modules": {},
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
        })).unwrap();
        cache.load_all(vec![guild.clone()]);
        assert!(cache.get("1").is_none());
        cache.set("1".into(), guild.clone());
        cache.remove("1");
        cache.load_all(vec![guild]);
        assert!(cache.get("1").is_none());
    }
}
