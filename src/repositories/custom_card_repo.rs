use crate::errors::Result;
use crate::models::CustomCard;
use sqlx::PgPool;

pub async fn create(
    pool: &PgPool,
    guild_id: &str,
    name: &str,
    title: &str,
    description: Option<&str>,
    image_url: Option<&str>,
    color: Option<&str>,
    footer: Option<&str>,
    created_by: &str,
) -> Result<CustomCard> {
    let card = sqlx::query_as::<_, CustomCard>(
        "INSERT INTO custom_cards (guild_id, name, title, description, image_url, color, footer, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (guild_id, name) DO UPDATE SET
             title = EXCLUDED.title,
             description = EXCLUDED.description,
             image_url = EXCLUDED.image_url,
             color = EXCLUDED.color,
             footer = EXCLUDED.footer,
             updated_at = NOW()
         RETURNING *"
    )
    .bind(guild_id)
    .bind(name)
    .bind(title)
    .bind(description)
    .bind(image_url)
    .bind(color)
    .bind(footer)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(card)
}

pub async fn find_by_name(pool: &PgPool, guild_id: &str, name: &str) -> Result<Option<CustomCard>> {
    let card = sqlx::query_as::<_, CustomCard>(
        "SELECT * FROM custom_cards WHERE guild_id = $1 AND name = $2"
    )
    .bind(guild_id)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(card)
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<CustomCard>> {
    let card = sqlx::query_as::<_, CustomCard>("SELECT * FROM custom_cards WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(card)
}

pub async fn list_by_guild(pool: &PgPool, guild_id: &str) -> Result<Vec<CustomCard>> {
    let cards = sqlx::query_as::<_, CustomCard>(
        "SELECT * FROM custom_cards WHERE guild_id = $1 ORDER BY name"
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;
    Ok(cards)
}

pub async fn delete(pool: &PgPool, guild_id: &str, name: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM custom_cards WHERE guild_id = $1 AND name = $2")
        .bind(guild_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update(
    pool: &PgPool,
    guild_id: &str,
    name: &str,
    title: Option<&str>,
    description: Option<&str>,
    image_url: Option<&str>,
    color: Option<&str>,
    footer: Option<&str>,
) -> Result<Option<CustomCard>> {
    let existing = find_by_name(pool, guild_id, name).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let card = sqlx::query_as::<_, CustomCard>(
        "UPDATE custom_cards SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            image_url = COALESCE($5, image_url),
            color = COALESCE($6, color),
            footer = COALESCE($7, footer),
            updated_at = NOW()
         WHERE guild_id = $1 AND name = $2
         RETURNING *"
    )
    .bind(guild_id)
    .bind(name)
    .bind(title)
    .bind(description)
    .bind(image_url)
    .bind(color)
    .bind(footer)
    .fetch_one(pool)
    .await?;
    Ok(Some(card))
}
