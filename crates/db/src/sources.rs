use anyhow::Result;

use sqlx::{Pool, Postgres, Transaction};

use crate::DBSource;

pub async fn create(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    source_link: &Option<String>,
    uploaded_by: i64,
    object_id: i64,
    version_id: Option<i64>,
    project_id: i64,
) -> Result<i64> {
    match sqlx::query!(
        "INSERT INTO sources (name, source_link, uploaded_by, object_id, version_id, project_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id",
        name,
        *source_link,
        uploaded_by,
        object_id,
        version_id,
        project_id
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(anyhow::Error::from)
    {
        Ok(r) => Ok(r.id),
        Err(e) => Err(e),
    }
}

pub async fn query_by_id(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> anyhow::Result<Option<DBSource>> {
    let row = sqlx::query_as!(DBSource, "SELECT * FROM sources WHERE id = $1", id)
        .fetch_optional(&mut **tx)
        .await?;

    Ok(row)
}

pub async fn query_by_slug(conn: Pool<Postgres>, query: &str) -> anyhow::Result<Option<DBSource>> {
    let sym = sqlx::query_as!(
        DBSource,
        "SELECT * FROM sources WHERE sources.slug = $1",
        query
    )
    .fetch_optional(&conn)
    .await?;

    Ok(sym)
}

pub async fn count(conn: Pool<Postgres>) -> Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM sources")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}
