CREATE TABLE IF NOT EXISTS lockdown_snapshots (
    guild_id TEXT PRIMARY KEY,
    channels JSONB NOT NULL CHECK (jsonb_typeof(channels) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE SEQUENCE IF NOT EXISTS lockdown_generation;
