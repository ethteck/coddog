use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use coddog_db::projects::CreateProjectRequest;
use coddog_db::symbols::QuerySymbolsByNameRequest;
use coddog_db::{SubmatchResult, SymbolMetadata};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::collections::{HashMap, HashSet};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

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

    // Set up CORS
    let cors_layer = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin("http://localhost:3001".parse::<HeaderValue>().unwrap())
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "coddog" }))
        .route("/projects", get(get_projects).post(create_project))
        .route(
            "/projects/{id}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .route("/symbols", post(query_symbols_by_name))
        .route("/symbols/{slug}", get(query_symbols_by_slug))
        .route("/symbols/{slug}/match", get(get_symbol_matches))
        .route("/symbols/{slug}/submatch", post(get_symbol_submatches))
        .with_state(db_pool)
        .layer(cors_layer);

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

    Ok((StatusCode::OK, json!(projects).to_string()))
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
    coddog_db::projects::update(pg_pool, id, &req)
        .await
        .map_err(|e| {
            eprintln!("Error updating project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::OK, json!(()).to_string()))
}

async fn delete_project(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    coddog_db::projects::delete(pg_pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error deleting project: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    Ok((StatusCode::NO_CONTENT, json!(()).to_string()))
}

async fn query_symbols_by_name(
    State(pg_pool): State<PgPool>,
    Json(req): Json<QuerySymbolsByNameRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let matches = coddog_db::symbols::query_by_name(pg_pool, &req)
        .await
        .map_err(|e| {
            eprintln!("Error fetching matches: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    let matches: Vec<SymbolMetadata> = matches.iter().map(SymbolMetadata::from_db_symbol).collect();

    Ok((StatusCode::OK, json!(matches).to_string()))
}

async fn query_symbols_by_slug(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let sym = coddog_db::symbols::query_by_slug(pg_pool.clone(), &slug)
        .await
        .map_err(|e| {
            eprintln!("Error fetching symbol by slug: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                json!({"success": false, "message": "Symbol not found"}).to_string(),
            )
        })?;

    Ok((
        StatusCode::OK,
        json!(SymbolMetadata::from_db_symbol(&sym)).to_string(),
    ))
}

async fn get_symbol_matches(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let query_sym = coddog_db::symbols::query_by_slug(pg_pool.clone(), &slug)
        .await
        .map_err(|e| {
            eprintln!("Error fetching symbol by slug: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                json!({"success": false, "message": "Symbol not found"}).to_string(),
            )
        })?;

    let mut found_stuff = HashSet::new();

    let exact_matches = coddog_db::symbols::query_by_exact_hash(pg_pool.clone(), &query_sym)
        .await
        .map_err(|e| {
            eprintln!("Error getting exact matches: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;
    found_stuff.extend(exact_matches.iter().map(|m| m.id));

    let mut equivalent_matches =
        coddog_db::symbols::query_by_equiv_hash(pg_pool.clone(), &query_sym)
            .await
            .map_err(|e| {
                eprintln!("Error getting equivalent matches: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"success": false, "message": e.to_string()}).to_string(),
                )
            })?;
    equivalent_matches.retain(|m| !found_stuff.contains(&m.id));
    found_stuff.extend(equivalent_matches.iter().map(|m| m.id));

    let mut opcode_matches = coddog_db::symbols::query_by_opcode_hash(pg_pool.clone(), &query_sym)
        .await
        .map_err(|e| {
            eprintln!("Error getting opcode matches: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;
    opcode_matches.retain(|m| !found_stuff.contains(&m.id));

    let query_sym = SymbolMetadata::from_db_symbol(&query_sym);
    let exact_matches: Vec<SymbolMetadata> = exact_matches
        .iter()
        .map(SymbolMetadata::from_db_symbol)
        .collect();
    let equivalent_matches: Vec<SymbolMetadata> = equivalent_matches
        .iter()
        .map(SymbolMetadata::from_db_symbol)
        .collect();
    let opcode_matches: Vec<SymbolMetadata> = opcode_matches
        .iter()
        .map(SymbolMetadata::from_db_symbol)
        .collect();

    Ok((
        StatusCode::OK,
        json!({"query": query_sym, "exact": exact_matches, "equivalent": equivalent_matches, "opcode": opcode_matches})
            .to_string(),
    ))
}

async fn get_symbol_submatches(
    State(pg_pool): State<PgPool>,
    Json(req): Json<coddog_db::symbols::QueryWindowsRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let db_window_size = std::env::var("DB_WINDOW_SIZE")
        .expect("DB_WINDOW_SIZE must be set")
        .parse::<i64>()
        .unwrap();

    if req.size <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({"success": false, "message": "size must be greater than 0"}).to_string(),
        ));
    }

    if req.page < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({"success": false, "message": "page must be at least 0"}).to_string(),
        ));
    }

    if req.size > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({"success": false, "message": "size must be 100 or less"}).to_string(),
        ));
    }

    if req.min_length < db_window_size {
        let msg = format!("min_length must be {} or greater", db_window_size);
        return Err((
            StatusCode::BAD_REQUEST,
            json!({"success": false, "message": msg}).to_string(),
        ));
    }

    let query_sym = coddog_db::symbols::query_by_slug(pg_pool.clone(), &req.slug)
        .await
        .map_err(|e| {
            eprintln!("Error fetching symbol by slug: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                json!({"success": false, "message": "Symbol not found"}).to_string(),
            )
        })?;

    let windows = coddog_db::query_windows_by_symbol_id(
        pg_pool.clone(),
        query_sym.id,
        req.min_length,
        db_window_size,
        req.size,
        req.page,
    )
    .await
    .map_err(|e| {
        eprintln!("Error fetching symbol by ID: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;

    let mut symbol_asm: HashMap<String, Vec<String>> = HashMap::new();
    for window in &windows {
        if !symbol_asm.contains_key(&window.symbol_slug) {
            let asm_text =
                coddog_core::get_asm_for_symbol(&window.object_path, window.object_symbol_idx)
                    .map_err(|e| {
                        eprintln!("Error getting ASM from bytes: {}", e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            json!({"success": false, "message": e.to_string()}).to_string(),
                        )
                    })?;
            symbol_asm.insert(window.symbol_slug.clone(), asm_text);
        }
    }

    let windows: Vec<SubmatchResult> = windows
        .into_iter()
        .map(|w| SubmatchResult::from_db_window(&w))
        .collect();
    let query_sym = SymbolMetadata::from_db_symbol(&query_sym);

    Ok((
        StatusCode::OK,
        json!({"query": query_sym, "submatches": windows, "asm": symbol_asm}).to_string(),
    ))
}
