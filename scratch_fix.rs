use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "postgres://bibliotecario:bibliotecario@localhost:5432/bibliotecario";
    let pool = PgPoolOptions::new().connect(url).await?;
    
    let result = sqlx::query!("UPDATE voice_sessions SET duration = 28800000 WHERE duration > 28800000")
        .execute(&pool)
        .await?;
        
    println!("Updated {} sessions that were too long", result.rows_affected());
    
    let result2 = sqlx::query!("UPDATE users u SET total_voice_time = COALESCE((SELECT SUM(vs.duration) FROM voice_sessions vs WHERE vs.user_id = u.user_id), 0)")
        .execute(&pool)
        .await?;
        
    println!("Updated {} users", result2.rows_affected());

    Ok(())
}
