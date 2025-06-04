use crate::Project;
use serde::Deserialize;
use sqlx::{Pool, Postgres};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub platform: i32,
    pub repo: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub platform: i32,
    pub repo: Option<String>,
}

pub async fn create(conn: Pool<Postgres>, request: &CreateProjectRequest) -> anyhow::Result<i64> {
    let rec = sqlx::query!(
        "INSERT INTO projects (name, platform, repo) VALUES ($1, $2, $3) RETURNING id",
        request.name,
        request.platform,
        request.repo
    )
    .fetch_one(&conn)
    .await?;

    Ok(rec.id)
}

pub async fn update(
    conn: Pool<Postgres>,
    id: i64,
    request: &UpdateProjectRequest,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE projects SET name = $1, platform = $2, repo = $3 WHERE id = $4",
        request.name,
        request.platform,
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
    let rows = sqlx::query_as!(
        Project,
        "SELECT * FROM projects WHERE projects.name LIKE $1",
        query
    )
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

pub async fn delete(conn: Pool<Postgres>, id: i64) -> anyhow::Result<()> {
    sqlx::query!("DELETE FROM projects WHERE id = $1", id)
        .execute(&conn)
        .await?;

    Ok(())
}
