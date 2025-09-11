use crate::Project;
use serde::Deserialize;
use sqlx::{Pool, Postgres, Transaction};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub repo: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub repo: Option<String>,
}

pub async fn create(
    tx: &mut Transaction<'_, Postgres>,
    request: &CreateProjectRequest,
) -> anyhow::Result<i64> {
    let rec = sqlx::query!(
        "INSERT INTO projects (name, repo) VALUES ($1, $2) RETURNING id",
        request.name,
        request.repo
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(rec.id)
}

pub async fn update(
    conn: Pool<Postgres>,
    id: i64,
    request: &UpdateProjectRequest,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE projects SET name = $1, repo = $2 WHERE id = $3",
        request.name,
        request.repo,
        id
    )
    .execute(&conn)
    .await?;

    Ok(())
}

pub async fn query_by_id(conn: Pool<Postgres>, id: i64) -> anyhow::Result<Option<Project>> {
    let row = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", id)
        .fetch_optional(&conn)
        .await?;

    Ok(row)
}

pub async fn query_by_name(conn: Pool<Postgres>, query: &str) -> anyhow::Result<Vec<Project>> {
    let rows = sqlx::query_as!(Project, "SELECT * FROM projects WHERE name LIKE $1", query)
        .fetch_all(&conn)
        .await?;

    Ok(rows)
}

pub async fn query_all(conn: Pool<Postgres>) -> anyhow::Result<Vec<Project>> {
    let rows = sqlx::query_as!(Project, "SELECT * FROM projects")
        .fetch_all(&conn)
        .await?;

    Ok(rows)
}

pub async fn delete(tx: &mut Transaction<'_, Postgres>, id: i64) -> anyhow::Result<()> {
    sqlx::query!("DELETE FROM projects WHERE id = $1", id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

pub async fn count(conn: Pool<Postgres>) -> anyhow::Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM projects")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}
