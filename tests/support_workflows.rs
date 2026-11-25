use bibliotecaria::services::support_state::{SupportStore, Key};

#[tokio::test]
#[ignore = "requires TASK5_DATABASE_URL pointing only at a disposable task5 database"]
async fn task5_persisted_workflow_failures_and_isolation() {
    let url = std::env::var("TASK5_DATABASE_URL").expect("isolated database required");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    bibliotecaria::database::run_migrations(&pool).await.unwrap();
    let store = SupportStore::new(pool.clone());
    let a = Key::new(51, 71); let b = Key::new(52, 71);
    let (one, two) = tokio::join!(store.reserve_ticket(a), store.reserve_ticket(a));
    assert_ne!(one.unwrap(), two.unwrap());
    assert!(store.reserve_ticket(b).await.unwrap());
    store.release_ticket(a).await.unwrap(); // confirmed creation failure
    assert!(store.reserve_ticket(a).await.unwrap());
    store.attach_ticket(a, 91).await.unwrap();
    let restarted = SupportStore::new(pool.clone());
    assert_eq!(restarted.ticket_owner(51, 91).await.unwrap(), Some(a));
    assert_eq!(restarted.ticket_owner(52, 91).await.unwrap(), None);
    assert!(!restarted.close_ticket(51, 91, 72, false, || async { Ok(()) }).await.unwrap());
    assert!(restarted.close_ticket(51, 91, 72, true, || async { Err(bibliotecaria::errors::BotError::Internal("delete failed".into())) }).await.is_err());
    assert_eq!(restarted.ticket_owner(51, 91).await.unwrap(), Some(a));
    assert!(restarted.close_ticket(51, 91, 72, true, || async { Ok(()) }).await.unwrap());
    assert_eq!(restarted.ticket_owner(51, 91).await.unwrap(), None);
    assert!(!restarted.reserve_ticket(b).await.unwrap());
    let (one,two) = tokio::join!(store.reserve_request(a), store.reserve_request(a));
    assert_ne!(one.unwrap(), two.unwrap());
    assert!(store.reserve_request(b).await.unwrap());
    store.attach_request(a, 101, 201).await.unwrap();
    assert!(!store.decide(a, 101, 202, || async { Ok(()) }).await.unwrap());
    assert!(store.decide(a, 101, 201, || async { Err(bibliotecaria::errors::BotError::Internal("role failed".into())) }).await.is_err());
    assert!(!store.reserve_request(a).await.unwrap());
    assert!(restarted.decide(a, 101, 201, || async { Ok(()) }).await.unwrap());
    assert!(!store.decide(a, 101, 201, || async { panic!("must not repeat role assignment") }).await.unwrap());
    sqlx::query("UPDATE access_requests SET created_at = NOW() - INTERVAL '24 hours' WHERE guild_id = '52'").execute(&pool).await.unwrap();
    assert!(store.reserve_request(b).await.unwrap());
    store.attach_request(b, 102, 202).await.unwrap();
    assert!(!store.decide(b, 101, 202, || async { panic!("wrong channel") }).await.unwrap());
    sqlx::query("UPDATE access_requests SET created_at = NOW() - INTERVAL '24 hours' WHERE guild_id = '52'").execute(&pool).await.unwrap();
    assert!(!store.decide(b, 102, 202, || async { panic!("expired request") }).await.unwrap());
    assert!(store.reserve_request(b).await.unwrap());
    store.attach_request(b, 102, 203).await.unwrap();
    assert!(!store.decide(b, 102, 202, || async { panic!("old message") }).await.unwrap());
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let effect = || async { calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst); Ok(()) };
    let (one,two) = tokio::join!(store.decide(b, 102, 203, effect), restarted.decide(b, 102, 203, effect));
    assert_ne!(one.unwrap(), two.unwrap());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(store.reserve_ticket(a).await.unwrap());
    store.attach_ticket(a, 92).await.unwrap();
    assert!(store.close_ticket(51, 92, 71, false, || async { Ok(()) }).await.unwrap());
    store.release_request(a).await.unwrap(); store.release_request(b).await.unwrap(); store.release_ticket(b).await.unwrap();
}
