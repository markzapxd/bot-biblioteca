-- Reservations without external IDs deliberately fail closed after an ambiguous crash.
-- Never infer ticket ownership from channel names or permission overwrites.
CREATE TABLE IF NOT EXISTS support_tickets (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    channel_id TEXT UNIQUE,
    PRIMARY KEY (guild_id, user_id)
);
CREATE TABLE IF NOT EXISTS access_requests (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    channel_id TEXT,
    message_id TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);
