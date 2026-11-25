use bibliotecaria::errors::BotError;
use bibliotecaria::services::lockdown::{Action, ChannelSnapshot, Store};
use serenity::all::*;
use std::sync::{Arc, Mutex};
#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn ownership_restart_partial_retry_and_stale_timer() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    let store = Store::new(pool.clone());
    let original = vec![PermissionOverwrite {
        kind: PermissionOverwriteType::Role(RoleId::new(1)),
        allow: Permissions::SEND_MESSAGES,
        deny: Permissions::VIEW_CHANNEL,
    }];
    let channels = vec![
        ChannelSnapshot::new(10, original.clone(), vec![]),
        ChannelSnapshot::new(11, vec![], original.clone()),
    ];
    let calls = Arc::new(Mutex::new(Vec::new()));
    let c = calls.clone();
    assert!(store
        .change(1, Action::Manual(true), channels.clone(), move |id, p| {
            let c = c.clone();
            async move {
                c.lock().unwrap().push((id, p));
                if id == 11 {
                    Err(BotError::Internal("injected".into()))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .is_err());
    let restarted = Store::new(pool.clone());
    let c = calls.clone();
    restarted
        .change(1, Action::RaidStart, vec![], move |id, p| {
            let c = c.clone();
            async move {
                c.lock().unwrap().push((id, p));
                Ok(())
            }
        })
        .await
        .unwrap();
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| *id == 10)
            .count(),
        1
    );
    restarted
        .change(1, Action::Manual(false), vec![], |_, _| async {
            panic!("raid still owns lockdown")
        })
        .await
        .unwrap();
    restarted
        .change(1, Action::RaidStart, vec![], |_, _| async {
            panic!("already applied")
        })
        .await
        .unwrap();
    restarted
        .change(1, Action::RaidEnd(u64::MAX), vec![], |_, _| async {
            panic!("old timer")
        })
        .await
        .unwrap();
    let restored = Arc::new(Mutex::new(Vec::new()));
    let r = restored.clone();
    restarted
        .change(1, Action::RaidDisable, vec![], move |id, p| {
            let r = r.clone();
            async move {
                r.lock().unwrap().push((id, p));
                Ok(())
            }
        })
        .await
        .unwrap();
    assert_eq!(restored.lock().unwrap().len(), 2);
    assert_eq!(restored.lock().unwrap()[0], (10, original));
    assert!(restarted
        .change(1, Action::Manual(false), vec![], |_, _| async {
            panic!("missing snapshot")
        })
        .await
        .is_err());
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn partial_restore_toggle_finishes_restore() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    let store = Store::new(pool);
    let original = vec![];
    let locked = vec![PermissionOverwrite {
        kind: PermissionOverwriteType::Role(RoleId::new(1)),
        allow: Permissions::empty(),
        deny: Permissions::SEND_MESSAGES,
    }];
    store
        .change(
            991,
            Action::Manual(true),
            vec![ChannelSnapshot::new(1, original.clone(), locked)],
            |_, _| async { Ok(()) },
        )
        .await
        .unwrap();
    assert!(store
        .change(991, Action::Manual(false), vec![], |_, _| async {
            Err(BotError::Internal("restore failed".into()))
        })
        .await
        .is_err());
    assert!(!store
        .change(991, Action::ToggleManual, vec![], |_, p| async move {
            assert!(p.is_empty(), "must restore, never relock");
            Ok(())
        })
        .await
        .unwrap());
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn legacy_raid_without_expiry_is_reconciled() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO lockdown_snapshots(guild_id,channels) VALUES('992',$1) ON CONFLICT(guild_id) DO UPDATE SET channels=EXCLUDED.channels").bind(serde_json::json!({"manual":false,"raid":20,"channels":[]})).execute(&pool).await.unwrap();
    let store = Store::new(pool);
    assert!(!store
        .change(992, Action::Reconcile(0), vec![], |_, _| async { Ok(()) })
        .await
        .unwrap());
    assert!(!store.raid_active(992).await.unwrap());
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn migration006_upgrades_original005_and_allocates_epochs() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    // A dedicated schema models the old migration ledger without changing other tests.
    let schema = "task6_upgrade_original005";
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&pool)
        .await
        .unwrap();
    let isolated = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |c, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO task6_upgrade_original005")
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE bot_schema_migrations(version INT PRIMARY KEY); INSERT INTO bot_schema_migrations VALUES(1),(2),(3),(4),(5); CREATE TABLE lockdown_snapshots(guild_id TEXT PRIMARY KEY,channels JSONB NOT NULL,created_at TIMESTAMPTZ DEFAULT NOW())").execute(&isolated).await.unwrap();
    bibliotecaria::database::run_migrations(&isolated)
        .await
        .unwrap();
    assert!(Store::new(isolated)
        .change(801, Action::RaidStart, vec![], |_, _| async { Ok(()) })
        .await
        .unwrap());
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn allocated_epochs_survive_restart_deletion_and_legacy_manual() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    let start = |id| vec![ChannelSnapshot::new(id, vec![], vec![])];
    let epoch = || {
        let pool = pool.clone();
        async move {
            let value: serde_json::Value =
                sqlx::query_scalar("SELECT channels FROM lockdown_snapshots WHERE guild_id='802'")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            value["raid"].as_u64().unwrap()
        }
    };
    Store::new(pool.clone())
        .change(802, Action::RaidStart, start(1), |_, _| async { Ok(()) })
        .await
        .unwrap();
    let old = epoch().await;
    let restarted = Store::new(pool.clone());
    restarted
        .change(802, Action::RaidStart, vec![], |_, _| async {
            panic!("already applied")
        })
        .await
        .unwrap();
    let new = epoch().await;
    assert!(new > old);
    restarted
        .change(802, Action::RaidEnd(old), vec![], |_, _| async {
            panic!("old allocated epoch")
        })
        .await
        .unwrap();
    restarted
        .change(802, Action::RaidDisable, vec![], |_, _| async { Ok(()) })
        .await
        .unwrap();
    Store::new(pool.clone())
        .change(802, Action::RaidStart, start(1), |_, _| async { Ok(()) })
        .await
        .unwrap();
    assert!(epoch().await > new);
    Store::new(pool.clone())
        .change(802, Action::RaidEnd(new), vec![], |_, _| async {
            panic!("deleted snapshot epoch")
        })
        .await
        .unwrap();
    sqlx::query("UPDATE lockdown_snapshots SET channels=jsonb_set(jsonb_set(channels,'{manual}','true'),'{expires}','null') WHERE guild_id='802'").execute(&pool).await.unwrap();
    let restarted = Store::new(pool.clone());
    assert!(restarted
        .change(802, Action::Reconcile(0), vec![], |_, _| async {
            panic!("manual owner must preserve applied channel")
        })
        .await
        .unwrap());
    assert!(!restarted.raid_active(802).await.unwrap());
    restarted
        .change(802, Action::Manual(false), vec![], |_, _| async { Ok(()) })
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn restore_failure_does_not_starve_and_success_is_not_retried() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    let store = Store::new(pool.clone());
    store
        .change(
            81001,
            Action::Manual(true),
            vec![
                ChannelSnapshot::new(1, vec![], vec![]),
                ChannelSnapshot::new(2, vec![], vec![]),
            ],
            |_, _| async { Ok(()) },
        )
        .await
        .unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let c = calls.clone();
    assert!(store
        .change(81001, Action::Manual(false), vec![], move |id, _| {
            c.lock().unwrap().push(id);
            async move {
                if id == 1 {
                    Err(BotError::Internal("transient".into()))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .is_err());
    assert_eq!(*calls.lock().unwrap(), vec![1, 2]);
    let restarted = Store::new(pool);
    restarted
        .change(81001, Action::Manual(false), vec![], |id, _| async move {
            assert_eq!(id, 1);
            Ok(())
        })
        .await
        .unwrap();
}

async fn discord_failure(status: u16, code: i64) -> BotError {
    let response = axum::http::Response::builder()
        .status(status)
        .body(format!(r#"{{"code":{code},"message":"injected"}}"#))
        .unwrap();
    let error =
        serenity::http::ErrorResponse::from_response(response.into(), reqwest::Method::PATCH).await;
    BotError::Discord(serenity::Error::Http(
        serenity::http::HttpError::UnsuccessfulRequest(error),
    ))
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn deleted_channels_are_terminal_only_for_exact_unknown_channel() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    for (guild, status, code, terminal) in [
        (82001, 404, 10003, true),
        (82002, 404, 10004, false),
        (82003, 403, 10003, false),
        (82004, 500, 10003, false),
    ] {
        let store = Store::new(pool.clone());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let c = calls.clone();
        let result = store
            .change(
                guild,
                Action::Manual(true),
                vec![
                    ChannelSnapshot::new(1, vec![], vec![]),
                    ChannelSnapshot::new(2, vec![], vec![]),
                ],
                move |id, _| {
                    c.lock().unwrap().push(id);
                    async move {
                        if id == 1 {
                            Err(discord_failure(status, code).await)
                        } else {
                            Ok(())
                        }
                    }
                },
            )
            .await;
        assert_eq!(result.is_ok(), terminal);
        assert_eq!(*calls.lock().unwrap(), vec![1, 2]);
        Store::new(pool.clone())
            .change(
                guild,
                Action::Manual(false),
                vec![],
                move |id, _| async move {
                    if terminal {
                        assert_eq!(id, 2);
                    }
                    Ok(())
                },
            )
            .await
            .unwrap();
    }
    let store = Store::new(pool.clone());
    store
        .change(
            82005,
            Action::Manual(true),
            vec![
                ChannelSnapshot::new(1, vec![], vec![]),
                ChannelSnapshot::new(2, vec![], vec![]),
            ],
            |_, _| async { Ok(()) },
        )
        .await
        .unwrap();
    store
        .change(82005, Action::Manual(false), vec![], |id, _| async move {
            if id == 1 {
                Err(discord_failure(404, 10003).await)
            } else {
                Ok(())
            }
        })
        .await
        .unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lockdown_snapshots WHERE guild_id='82005'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
#[ignore = "requires TASK6_DATABASE_URL disposable postgres"]
async fn stored_private_avatar_is_scoped_in_both_production_renderers() {
    use bibliotecaria::{
        repositories::user_repo::{self, HistoryKind},
        services::{avatar_manager, history_manager},
    };
    let pool = sqlx::PgPool::connect(&std::env::var("TASK6_DATABASE_URL").unwrap())
        .await
        .unwrap();
    bibliotecaria::database::run_migrations(&pool)
        .await
        .unwrap();
    for (url, guild) in [
        ("global", None),
        ("secret-a", Some("1")),
        ("local-b", Some("2")),
    ] {
        let mut entry = serde_json::json!({"url":url,"date":"2026-01-01T00:00:00Z"});
        if let Some(guild) = guild {
            entry["guild_id"] = guild.into();
        }
        user_repo::append_history(&pool, "final-avatar", HistoryKind::Avatar, entry)
            .await
            .unwrap();
    }
    user_repo::update_privacy(&pool, "final-avatar", true)
        .await
        .unwrap();
    let user = user_repo::find_by_id(&pool, "final-avatar")
        .await
        .unwrap()
        .unwrap();
    let entries = user.get_avatar_history();
    for guild in [None, Some(GuildId::new(2))] {
        for page in 0..4 {
            for embed in [
                avatar_manager::build_avatar_page(1, &entries, page, guild).0,
                history_manager::build_avatars_page(1, &entries, page, guild).0,
            ] {
                let rendered = serde_json::to_string(&embed).unwrap();
                assert!(!rendered.contains("secret-a"));
                if guild.is_none() {
                    assert!(!rendered.contains("local-b"));
                }
            }
        }
    }
}
