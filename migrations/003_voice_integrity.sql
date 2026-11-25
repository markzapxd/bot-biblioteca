-- Keep the newest active session, deterministically breaking timestamp ties by id.
WITH ranked AS (
 SELECT id, row_number() OVER (PARTITION BY user_id, guild_id ORDER BY joined_at DESC, id DESC) AS n
 FROM voice_sessions WHERE active = TRUE
)
UPDATE voice_sessions v SET active = FALSE,
 left_at = GREATEST(v.joined_at, v.updated_at),
 duration = GREATEST(0, (EXTRACT(EPOCH FROM (v.updated_at - v.joined_at)) * 1000)::bigint)
FROM ranked r WHERE v.id = r.id AND r.n > 1;
CREATE UNIQUE INDEX voice_sessions_one_active ON voice_sessions(user_id, guild_id) WHERE active = TRUE;
INSERT INTO users(user_id) SELECT DISTINCT user_id FROM voice_sessions ON CONFLICT DO NOTHING;
UPDATE users u SET total_voice_time = COALESCE((SELECT SUM(duration) FROM voice_sessions v WHERE v.user_id = u.user_id), 0);
