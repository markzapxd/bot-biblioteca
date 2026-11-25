#[cfg(test)]
mod model_tests {
    use crate::models::*;

    #[test]
    fn test_guild_modules_default() {
        let modules = GuildModules::default();
        assert!(modules.antiraid);
        assert!(modules.logs);
        assert!(modules.tickets);
    }

    #[test]
    fn test_voice_session_duration_formatted() {
        let session = VoiceSession {
            id: 1,
            user_id: "123".into(),
            guild_id: "456".into(),
            guild_name: None,
            channel_id: "789".into(),
            channel_name: "test".into(),
            joined_at: chrono::Utc::now(),
            left_at: None,
            duration: Some(3661000),
            members_at_end: serde_json::json!([]),
            active: Some(false),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(session.duration_formatted(), "1h 1m 1s");
    }

    #[test]
    fn test_user_privacy_default() {
        let user = User {
            user_id: "123".into(),
            is_private: None,
            total_voice_time: Some(0),
            premium: None,
            username_history: serde_json::json!([]),
            avatar_history: serde_json::json!([]),
            nickname_history: serde_json::json!([]),
            last_seen: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert!(!user.is_private_mode());
    }
}

#[cfg(test)]
mod time_tests {
    use crate::utils::time;

    #[test]
    fn test_format_duration() {
        assert_eq!(time::format_duration(3661000), "1h 1m 1s");
        assert_eq!(time::format_duration(60000), "1m");
        assert_eq!(time::format_duration(0), "0s");
    }
}

#[cfg(test)]
mod permission_tests {
    use crate::permissions;

    #[test]
    fn test_is_owner() {
        let configured = std::env::var("OWNER_ID").ok().and_then(|v| v.parse::<u64>().ok()).filter(|id| *id != 0);
        assert_eq!(permissions::is_owner(852879300336418837), configured == Some(852879300336418837));
        assert!(!permissions::is_owner(0));
    }
}

#[cfg(test)]
mod db_tests {
    use sqlx::PgPool;

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL pointing to an isolated test database"]
    async fn print_db_cards() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must point to an isolated test database");
        let pool = PgPool::connect(&database_url)
            .await
            .unwrap_or_else(|_| panic!("failed to connect to isolated test database"));
        let rows: Vec<crate::models::CustomCard> = sqlx::query_as("SELECT * FROM custom_cards")
            .fetch_all(&pool)
            .await
            .unwrap_or_else(|_| panic!("failed to query custom_cards in isolated test database"));
        for r in rows {
            println!("=== CARD ===");
            println!("name: {}", r.name);
            println!("title: {}", r.title);
            println!("image_url: {:?}", r.image_url);
        }
    }
}
