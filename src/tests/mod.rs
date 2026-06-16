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
        assert_eq!(time::format_duration(60000), "1m 0s");
        assert_eq!(time::format_duration(0), "0s");
    }
}

#[cfg(test)]
mod permission_tests {
    use crate::permissions;

    #[test]
    fn test_is_owner() {
        assert!(permissions::is_owner(852879300336418837));
        assert!(!permissions::is_owner(123456789));
    }
}
