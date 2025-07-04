use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use coddog_db::projects::CreateProjectRequest;
use serde_json::json;
use sqlx::PgPool;

pub(crate) async fn get_projects(
    State(pg_pool): State<PgPool>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let projects = coddog_db::projects::query_all(pg_pool).await.map_err(|e| {
        eprintln!("Error fetching projects: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;

    Ok((StatusCode::OK, json!(projects).to_string()))
}

pub(crate) async fn create_project(
    State(pg_pool): State<PgPool>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let res = coddog_db::projects::create(pg_pool, &req)
        .await
        .map_err(|e| {
            eprintln!("Error creating project: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::CREATED, json!(res).to_string()))
}

pub(crate) async fn get_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let project = coddog_db::projects::query_by_id(pg_pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error fetching project: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::OK, json!(project).to_string()))
}

pub(crate) async fn update_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<coddog_db::projects::UpdateProjectRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    coddog_db::projects::update(pg_pool, id, &req)
        .await
        .map_err(|e| {
            eprintln!("Error updating project: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::OK, json!(()).to_string()))
}

pub(crate) async fn delete_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    coddog_db::projects::delete(pg_pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error deleting project: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::NO_CONTENT, json!(()).to_string()))
}
