CREATE TABLE IF NOT EXISTS guilds (
    guild_id VARCHAR(255) PRIMARY KEY,
    prefix VARCHAR(255) DEFAULT '!',
    member_role_id VARCHAR(255),
    staff_channel_id VARCHAR(255),
    welcome_channel_id VARCHAR(255),
    log_channel_id VARCHAR(255),
    admin_role_id VARCHAR(255),
    staff_role_id VARCHAR(255),
    ticket_category_id VARCHAR(255),
    frin_monitor_channel_id VARCHAR(255),
    modules JSONB DEFAULT '{}'::jsonb,
    webhook_url VARCHAR(512),
    premium BOOLEAN DEFAULT FALSE,
    track_mute BOOLEAN DEFAULT FALSE,
    track_deaf BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
    user_id VARCHAR(255) PRIMARY KEY,
    is_private BOOLEAN DEFAULT FALSE,
    total_voice_time BIGINT DEFAULT 0,
    premium BOOLEAN DEFAULT FALSE,
    username_history JSONB DEFAULT '[]'::jsonb,
    avatar_history JSONB DEFAULT '[]'::jsonb,
    last_seen TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS voice_sessions (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    guild_id VARCHAR(255) NOT NULL,
    guild_name VARCHAR(255),
    channel_id VARCHAR(255) NOT NULL,
    channel_name VARCHAR(255) NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL,
    left_at TIMESTAMPTZ,
    duration BIGINT DEFAULT 0,
    members_at_end JSONB DEFAULT '[]'::jsonb,
    active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_voice_sessions_user_joined ON voice_sessions(user_id, joined_at);
CREATE INDEX IF NOT EXISTS idx_voice_sessions_guild_duration ON voice_sessions(guild_id, duration);
CREATE INDEX IF NOT EXISTS idx_voice_sessions_active ON voice_sessions(active) WHERE active = TRUE;

ALTER TABLE users ADD COLUMN IF NOT EXISTS nickname_history JSONB DEFAULT '[]'::jsonb;
