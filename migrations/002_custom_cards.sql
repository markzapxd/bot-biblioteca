CREATE TABLE IF NOT EXISTS custom_cards (
    id SERIAL PRIMARY KEY,
    guild_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    title VARCHAR(512) NOT NULL,
    description TEXT,
    image_url VARCHAR(1024),
    color VARCHAR(16),
    footer VARCHAR(512),
    created_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, name)
);

CREATE INDEX IF NOT EXISTS idx_custom_cards_guild ON custom_cards(guild_id);
CREATE INDEX IF NOT EXISTS idx_custom_cards_name ON custom_cards(guild_id, name);