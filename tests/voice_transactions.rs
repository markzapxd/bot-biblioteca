use bibliotecaria::{database, repositories::{user_repo, voice_session_repo as voice}};
use chrono::Utc;
use sqlx::PgPool;

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn avatar_history_deduplicates_within_each_provenance() {
    let pool = pool().await;
    database::run_migrations(&pool).await.unwrap();
    let user = uuid::Uuid::new_v4().to_string();
    for guild in [Some("A"), None, Some("B"), Some("A"), None, Some("B")] {
        let mut entry = serde_json::json!({"url":"identical", "date":Utc::now()});
        if let Some(guild) = guild { entry["guild_id"] = guild.into(); }
        user_repo::append_history(&pool, &user, user_repo::HistoryKind::Avatar, entry).await.unwrap();
    }
    let row = user_repo::find_by_id(&pool, &user).await.unwrap().unwrap();
    assert_eq!(row.avatar_history.as_array().unwrap().len(), 3);
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn migration_deduplicates_once_and_preserves_legacy_history() {
    let pool = pool().await;
    let schema = format!("task4_{}", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}")).execute(&pool).await.unwrap();
    let options = pool.connect_options().as_ref().clone().options([("search_path", schema.as_str())]);
    let isolated = sqlx::postgres::PgPoolOptions::new().connect_with(options).await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/001_initial_schema.sql")).execute(&isolated).await.unwrap();
    sqlx::query("INSERT INTO users(user_id,username_history) VALUES ('legacy', (SELECT jsonb_agg(i) FROM generate_series(1,600) i))").execute(&isolated).await.unwrap();
    sqlx::query("INSERT INTO voice_sessions(user_id,guild_id,channel_id,channel_name,joined_at) VALUES ('legacy','g','a','a',NOW()), ('legacy','g','b','b',NOW())").execute(&isolated).await.unwrap();
    database::run_migrations(&isolated).await.unwrap();
    let active = voice::find_active_by_user_guild(&isolated,"legacy","g").await.unwrap().unwrap();
    assert_eq!(active.channel_id,"b");
    sqlx::query("UPDATE users SET total_voice_time=123 WHERE user_id='legacy'").execute(&isolated).await.unwrap();
    let (a,b) = tokio::join!(database::run_migrations(&isolated),database::run_migrations(&isolated));
    a.unwrap(); b.unwrap();
    let user = user_repo::find_by_id(&isolated,"legacy").await.unwrap().unwrap();
    assert_eq!(user.username_history.as_array().unwrap().len(),600);
    assert_eq!(user.total_voice_time,Some(123));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM bot_schema_migrations").fetch_one(&isolated).await.unwrap(),6);
    isolated.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE")).execute(&pool).await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn cache_guild_initialization_is_repeatable_and_scoped() {
    use serenity::all::*;
    let pool = pool().await;
    let cache = serenity::cache::Cache::new();
    let user = uuid::Uuid::new_v4().to_string();
    voice::transition(&pool,&user,"992",None,Some(("other","other")),Utc::now()).await.unwrap();
    let mut guild = Guild::default();
    guild.id = GuildId::new(991);
    guild.name = "fixture".into();
    let state: VoiceState = serde_json::from_value(serde_json::json!({"user_id":"1234", "channel_id":"4567", "guild_id":"991", "session_id":"fixture", "deaf":false,"mute":false,"self_deaf":false,"self_mute":false,"self_video":false,"suppress":false})).unwrap();
    guild.voice_states.insert(UserId::new(1234),state);
    let mut event: GuildCreateEvent = serde_json::from_value(serde_json::to_value(guild).unwrap()).unwrap();
    cache.update(&mut event);
    for _ in 0..2 { bibliotecaria::jobs::voice_sync::initialize_cached_guild(&cache,&pool,GuildId::new(991)).await.unwrap(); }
    let before = voice::find_active_by_user_guild(&pool,"1234","991").await.unwrap().unwrap();
    bibliotecaria::jobs::voice_sync::initialize_cached_guild(&cache,&pool,GuildId::new(991)).await.unwrap();
    assert_eq!(before.id,voice::find_active_by_user_guild(&pool,"1234","991").await.unwrap().unwrap().id);
    assert!(voice::find_active_by_user_guild(&pool,&user,"992").await.unwrap().is_some());
    let mut new_guild = Guild::default(); new_guild.id=GuildId::new(993);
    let mut event: GuildCreateEvent=serde_json::from_value(serde_json::to_value(new_guild).unwrap()).unwrap();
    cache.update(&mut event);
    voice::transition(&pool,&user,"993",None,Some(("stale","stale")),Utc::now()).await.unwrap();
    bibliotecaria::jobs::voice_sync::initialize_cached_guild(&cache,&pool,GuildId::new(993)).await.unwrap();
    assert!(voice::find_active_by_user_guild(&pool,&user,"993").await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn blocked_recompute_reads_sum_after_transition_commits() {
    let pool = pool().await;
    for all in [false, true] {
        let user = uuid::Uuid::new_v4().to_string();
        voice::transition(&pool,&user,"g",None,Some(("c","c")),Utc::now()).await.unwrap();
        // Hold precisely the user-row lock acquired by transition before closing.
        let mut leave = pool.begin().await.unwrap();
        let holder: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *leave).await.unwrap();
        sqlx::query("SELECT user_id FROM users WHERE user_id=$1 FOR UPDATE").bind(&user).execute(&mut *leave).await.unwrap();
        sqlx::query("UPDATE voice_sessions SET active=FALSE, duration=424242, left_at=NOW() WHERE user_id=$1").bind(&user).execute(&mut *leave).await.unwrap();
        sqlx::query("UPDATE users SET total_voice_time=424242 WHERE user_id=$1").bind(&user).execute(&mut *leave).await.unwrap();
        let worker_pool = sqlx::postgres::PgPoolOptions::new().max_connections(1).connect_with(pool.connect_options().as_ref().clone()).await.unwrap();
        let worker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&worker_pool).await.unwrap();
        let p = worker_pool.clone(); let u = user.clone();
        let worker = tokio::spawn(async move {
            if all { user_repo::recompute_all_voice_times(&p).await.unwrap(); }
            else { user_repo::recompute_voice_time(&p,&u).await.unwrap(); }
        });
        // Observe the actual PostgreSQL wait edge, not timing guesses.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let blocked: bool = sqlx::query_scalar("SELECT $1 = ANY(pg_blocking_pids($2))").bind(holder).bind(worker_pid).fetch_one(&pool).await.unwrap();
                if blocked { break; }
                tokio::task::yield_now().await;
            }
        }).await.expect("worker must wait on transition's lock");
        leave.commit().await.unwrap();
        worker.await.unwrap();
        assert_eq!(user_repo::find_by_id(&pool,&user).await.unwrap().unwrap().total_voice_time,Some(424242),"all={all}: worker must not overwrite committed aggregate with pre-lock snapshot");
        worker_pool.close().await;
    }
}

// Explicit isolated URL only; never falls back to DATABASE_URL or loads dotenv.
async fn pool() -> PgPool {
    let url = std::env::var("TASK4_TEST_DATABASE_URL").expect("explicit disposable DB required");
    let pool = database::create_pool(&url).await.unwrap();
    database::run_migrations(&pool).await.unwrap();
    pool
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn active_uniqueness_is_enforced_by_database() {
    let pool = pool().await;
    let user = uuid::Uuid::new_v4().to_string();
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let p = pool.clone(); let u = user.clone();
        tasks.push(tokio::spawn(async move { voice::create(&p, &u, "g", None, "c", "channel", Utc::now()).await }));
    }
    let mut successes = 0;
    for task in tasks { if task.await.unwrap().is_ok() { successes += 1; } }
    assert_eq!(successes, 1, "database must reject duplicate active sessions");
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn concurrent_transitions_upsert_user_and_rollback_failed_move() {
    let pool = pool().await;
    let user = uuid::Uuid::new_v4().to_string();
    let mut tasks = Vec::new();
    for i in 0..16 {
        let p = pool.clone(); let u = user.clone();
        tasks.push(tokio::spawn(async move {
            let channel = format!("c{i}");
            voice::transition(&p, &u, "g", None, Some((&channel, "channel")), Utc::now()).await.unwrap();
        }));
    }
    for task in tasks { task.await.unwrap(); }
    let before = voice::find_active_by_user_guild(&pool, &user, "g").await.unwrap().unwrap();
    assert!(user_repo::find_by_id(&pool, &user).await.unwrap().is_some());
    let invalid = "x".repeat(256);
    assert!(voice::transition(&pool, &user, "g", None, Some((&invalid, "invalid")), Utc::now()).await.is_err());
    let after = voice::find_active_by_user_guild(&pool, &user, "g").await.unwrap().unwrap();
    assert_eq!(before.id, after.id);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM voice_sessions WHERE user_id=$1 AND active").bind(&user).fetch_one(&pool).await.unwrap(), 1);
    voice::transition(&pool, &user, "other", None, Some(("other", "other")), Utc::now()).await.unwrap();
    voice::transition(&pool, &user, "g", None, None, Utc::now()).await.unwrap();
    assert!(voice::find_active_by_user_guild(&pool, &user, "other").await.unwrap().is_some());
}

#[tokio::test]
#[ignore = "requires explicit disposable PostgreSQL"]
async fn concurrent_history_append_is_lossless_bounded_and_deduplicated() {
    let pool = pool().await;
    let user = uuid::Uuid::new_v4().to_string();
    let mut tasks = Vec::new();
    for i in 0..40 {
        let p = pool.clone(); let u = user.clone();
        tasks.push(tokio::spawn(async move { user_repo::append_history(&p, &u, user_repo::HistoryKind::Nickname, serde_json::json!({"name": i.to_string(), "guild_id":"g", "date": Utc::now()})).await.unwrap(); }));
    }
    for task in tasks { task.await.unwrap(); }
    let data = user_repo::find_by_id(&pool, &user).await.unwrap().unwrap();
    assert_eq!(data.get_nickname_history().len(), 40);
    assert!(data.get_nickname_history().iter().all(|entry| entry.guild_id.as_deref() == Some("g")));
    for i in 0..510 {
        user_repo::append_history(&pool, &user, user_repo::HistoryKind::Username, serde_json::json!({"name":i.to_string(), "date":Utc::now()})).await.unwrap();
    }
    let entry = serde_json::json!({"name":"509", "date":Utc::now()});
    user_repo::append_history(&pool, &user, user_repo::HistoryKind::Username, entry.clone()).await.unwrap();
    user_repo::append_history(&pool, &user, user_repo::HistoryKind::Username, entry).await.unwrap();
    let data = user_repo::find_by_id(&pool, &user).await.unwrap().unwrap();
    let names = data.get_username_history();
    assert_eq!(names.len(), 500);
    assert_eq!(names.first().unwrap().name, "10");
    assert_eq!(names.last().unwrap().name, "509");
    assert_eq!(data.get_nickname_history().len(), 40);
}

