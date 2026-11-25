use bibliotecaria::repositories::{custom_card_repo as cards, guild_repo};

#[tokio::test]
#[ignore = "requires TASK7_DATABASE_URL pointing only at an owned disposable database"]
async fn task7_atomic_rename_and_typed_booleans() {
    let pool = sqlx::PgPool::connect(&std::env::var("TASK7_DATABASE_URL").unwrap()).await.unwrap();
    bibliotecaria::database::run_migrations(&pool).await.unwrap();
    guild_repo::upsert(&pool, "7").await.unwrap();
    guild_repo::upsert(&pool, "8").await.unwrap();
    let a = cards::create(&pool, "7", "a", "original", None, None, None, None, "author").await.unwrap();
    let b = cards::create(&pool, "7", "b", "destination", None, None, None, None, "other").await.unwrap();
    assert!(cards::replace(&pool, "7", a.id, "b", "changed", None, None, None).await.is_err());
    assert_eq!(cards::find_by_guild_id(&pool, "7", a.id).await.unwrap().unwrap().title, "original");
    assert_eq!(cards::find_by_guild_id(&pool, "7", b.id).await.unwrap().unwrap().title, "destination");
    assert!(cards::create(&pool, "7", "b", "overwrite", None, None, None, None, "x").await.is_err());
    assert!(cards::replace(&pool, "8", a.id, "stolen", "bad", None, None, None).await.unwrap().is_none());
    let renamed = cards::replace(&pool, "7", a.id, "renamed", "updated", Some(""), None, None).await.unwrap().unwrap();
    assert_eq!(renamed.id, a.id);
    assert_eq!(renamed.created_by, a.created_by);
    assert_eq!(renamed.created_at, a.created_at);
    for field in ["premium", "track_mute", "track_deaf"] {
        for value in ["true", "false"] {
            guild_repo::update_field(&pool, "7", field, value).await.unwrap();
            let actual: bool = sqlx::query_scalar(&format!("SELECT {field} FROM guilds WHERE guild_id='7'"))
                .fetch_one(&pool).await.unwrap();
            assert_eq!(actual, value == "true");
        }
        assert!(matches!(guild_repo::update_field(&pool, "7", field, "yes").await,
            Err(bibliotecaria::errors::BotError::Validation(_))));
    }
}
