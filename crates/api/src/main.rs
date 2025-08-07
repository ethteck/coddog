mod projects;

use crate::projects::{create_project, delete_project, get_project, get_projects, update_project};
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_validated_extractors::ValidatedJson;
use coddog_db::symbols::QuerySymbolsByNameRequest;
use coddog_db::{DBSymbol, QueryWindowsRequest, SubmatchResult, SymbolMetadata};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use validator::{Validate, ValidationError};

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

    println!("Listening on {server_address}");

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
        .route("/symbols/{slug}/asm", get(get_symbol_asm))
        .route("/symbols/{slug}/match", get(get_symbol_matches))
        .route("/symbols/{slug}/submatch", post(get_symbol_submatches))
        .with_state(db_pool)
        .layer(cors_layer);

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn query_symbols_by_name(
    State(pg_pool): State<PgPool>,
    Json(req): Json<QuerySymbolsByNameRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let matches = coddog_db::symbols::query_by_name(pg_pool, &req)
        .await
        .map_err(|e| {
            eprintln!("Error fetching matches: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;

    let matches: Vec<SymbolMetadata> = matches.iter().map(SymbolMetadata::from_db_symbol).collect();

    Ok((StatusCode::OK, json!(matches).to_string()))
}

async fn get_sym_for_slug(pg_pool: PgPool, slug: &str) -> Result<DBSymbol, (StatusCode, String)> {
    coddog_db::symbols::query_by_slug(pg_pool.clone(), slug)
        .await
        .map_err(|e| {
            eprintln!("Error fetching symbol by slug: {e}");
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
        })
}

async fn query_symbols_by_slug(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let sym = get_sym_for_slug(pg_pool, &slug).await?;

    Ok((
        StatusCode::OK,
        json!(SymbolMetadata::from_db_symbol(&sym)).to_string(),
    ))
}

fn get_asm_for_symbol(
    object_path: &str,
    symbol_idx: i32,
) -> Result<Vec<String>, (StatusCode, String)> {
    let asm_text = coddog_core::get_asm_for_symbol(object_path, symbol_idx).map_err(|e| {
        eprintln!("Error getting ASM from symbol {symbol_idx} in {object_path}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;
    Ok(asm_text)
}

async fn get_symbol_asm(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let sym = get_sym_for_slug(pg_pool.clone(), &slug).await?;

    let asm_text = get_asm_for_symbol(&sym.object_path, sym.object_symbol_idx)?;

    Ok((StatusCode::OK, json!({"asm": asm_text}).to_string()))
}

#[derive(Clone, Serialize)]
struct SymbolMatchResult {
    subtype: String,
    symbol: SymbolMetadata,
}

async fn get_symbol_matches(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let query_sym = get_sym_for_slug(pg_pool.clone(), &slug).await?;

    let mut found_stuff = HashSet::new();

    let exact_matches = coddog_db::symbols::query_by_exact_hash(pg_pool.clone(), &query_sym)
        .await
        .map_err(|e| {
            eprintln!("Error getting exact matches: {e}");
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
                eprintln!("Error getting equivalent matches: {e}");
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
            eprintln!("Error getting opcode matches: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success": false, "message": e.to_string()}).to_string(),
            )
        })?;
    opcode_matches.retain(|m| !found_stuff.contains(&m.id));

    let exact_matches: Vec<SymbolMatchResult> = exact_matches
        .iter()
        .map(|s| SymbolMatchResult {
            subtype: "exact".to_string(),
            symbol: SymbolMetadata::from_db_symbol(s),
        })
        .collect();
    let equivalent_matches: Vec<SymbolMatchResult> = equivalent_matches
        .iter()
        .map(|s| SymbolMatchResult {
            subtype: "equivalent".to_string(),
            symbol: SymbolMetadata::from_db_symbol(s),
        })
        .collect();
    let opcode_matches: Vec<SymbolMatchResult> = opcode_matches
        .iter()
        .map(|s| SymbolMatchResult {
            subtype: "opcode".to_string(),
            symbol: SymbolMetadata::from_db_symbol(s),
        })
        .collect();

    let all_matches: Vec<SymbolMatchResult> = exact_matches
        .iter()
        .chain(equivalent_matches.iter())
        .chain(opcode_matches.iter())
        .cloned()
        .collect();

    Ok((StatusCode::OK, json!(all_matches).to_string()))
}

#[derive(Deserialize, Validate)]
struct GetSubmatchesRequest {
    #[validate(custom(function = "validate_window_size"))]
    pub window_size: i64,
    #[validate(range(min = 0))]
    pub start: Option<i64>,
    #[validate(range(min = 0))]
    pub end: Option<i64>,
    #[validate(range(min = 0))]
    pub page_num: i64,
    #[validate(range(min = 1, max = 100))]
    pub page_size: i64,
}

fn validate_window_size(input: i64) -> Result<(), ValidationError> {
    let db_window_size = std::env::var("DB_WINDOW_SIZE")
        .expect("DB_WINDOW_SIZE must be set")
        .parse::<i64>()
        .unwrap();

    if input < db_window_size {
        return Err(ValidationError::new(
            "window_size must be greater than or equal to 8",
        ));
    }

    Ok(())
}

async fn get_symbol_submatches(
    State(pg_pool): State<PgPool>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    ValidatedJson(req): ValidatedJson<GetSubmatchesRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let db_window_size = std::env::var("DB_WINDOW_SIZE")
        .expect("DB_WINDOW_SIZE must be set")
        .parse::<i64>()
        .unwrap();

    let query_sym = coddog_db::symbols::query_by_slug(pg_pool.clone(), &slug)
        .await
        .map_err(|e| {
            eprintln!("Error fetching symbol by slug: {e}");
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

    let start = req.start.unwrap_or(0) as i32;
    let end = req.end.unwrap_or(query_sym.get_num_insns().into()) as i32;

    let windows_results = coddog_db::query_windows_by_symbol_id(
        pg_pool.clone(),
        QueryWindowsRequest {
            symbol_id: query_sym.id,
            start,
            end,
            window_size: req.window_size,
            db_window_size,
            limit: req.page_size,
            page: req.page_num,
        },
    )
    .await
    .map_err(|e| {
        eprintln!("Error fetching symbol by ID: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"success": false, "message": e.to_string()}).to_string(),
        )
    })?;

    // let mut symbol_asm: HashMap<String, Vec<String>> = HashMap::new();
    // for window in &windows {
    //     if !symbol_asm.contains_key(&window.symbol_slug) {
    //         let asm = get_asm_for_symbol(&window.object_path, window.object_symbol_idx)?;
    //         symbol_asm.insert(window.symbol_slug.clone(), asm);
    //     }
    // }
    //
    // // add query symbol asm if not already present
    // if !symbol_asm.contains_key(&query_sym.slug) {
    //     let asm = get_asm_for_symbol(&query_sym.object_path, query_sym.object_symbol_idx)?;
    //     symbol_asm.insert(query_sym.slug.clone(), asm);
    // }

    let windows: Vec<SubmatchResult> = windows_results
        .windows
        .into_iter()
        .map(|w| SubmatchResult::from_db_window(&w))
        .collect();

    Ok((
        StatusCode::OK,
        json!({"submatches": windows, "total_count": windows_results.total_count}).to_string(),
    ))
}
