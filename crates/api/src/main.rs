use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use coddog_db::projects::CreateProjectRequest;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("No .env file found");

    let server_address = std::env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:3000".to_string());
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    let listener = TcpListener::bind(&server_address)
        .await
        .expect("Could not bind to server address");

    println!("Listening on {}", server_address);

    let app = Router::new()
        .route("/", get(|| async { "coddog" }))
        .route("/projects", get(get_projects).post(create_project))
        .route(
            "/projects/{id}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .with_state(db_pool);

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn get_projects(
    State(pg_pool): State<PgPool>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let projects = coddog_db::projects::query_all(pg_pool).await.map_err(|e| {
        eprintln!("Error fetching projects: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;

    Ok((
        StatusCode::OK,
        json!({"success": true, "projects": projects}).to_string(),
    ))
}

async fn create_project(
    State(pg_pool): State<PgPool>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let res = coddog_db::projects::create(pg_pool, &req)
        .await
        .map_err(|e| {
            eprintln!("Error creating project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::CREATED, json!(res).to_string()))
}

async fn get_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let project = coddog_db::projects::query_by_id(pg_pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error fetching project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::OK, json!(project).to_string()))
}

async fn update_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<coddog_db::projects::UpdateProjectRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let res = coddog_db::projects::update(pg_pool, id, &req)
        .await
        .map_err(|e| {
            eprintln!("Error updating project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::OK, json!(res).to_string()))
}

async fn delete_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let res = coddog_db::projects::delete(pg_pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error deleting project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::NO_CONTENT, json!(res).to_string()))
}
