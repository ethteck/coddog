pub mod decompme;
pub mod objects;
pub mod projects;
pub mod symbols;

use anyhow::Result;
use coddog_core::Platform;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Pool, Postgres, Transaction, migrate::MigrateDatabase};
use std::fmt::{Display, Formatter};

const CHUNK_SIZE: usize = 100000;

#[derive(Clone, Debug, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub repo: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Version {
    pub id: i64,
    pub name: String,
    pub platform: i32,
    pub project_id: i64,
}

impl Display for Project {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.name))
    }
}

#[derive(Clone, Debug)]
pub struct DBSymbol {
    pub id: i64,
    pub slug: String,
    pub len: i32,
    pub name: String,
    pub is_decompiled: bool,
    pub symbol_idx: i32,
    pub opcode_hash: i64,
    pub equiv_hash: i64,
    pub exact_hash: i64,
    pub source_id: i64,
    pub source_name: String,
    pub object_path: String,
    pub object_symbol_idx: i32,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub project_id: i64,
    pub project_name: String,
    pub project_repo: Option<String>,
    pub platform: i32,
}

impl Display for DBSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} version {} (idx {:X})",
            self.project_name, self.source_name, self.object_symbol_idx,
        ))
    }
}

impl DBSymbol {
    pub fn get_num_insns(&self) -> i32 {
        let platform: Platform = self.platform.try_into().expect("Unexpected platform ID");
        self.len / platform.arch().standard_insn_length() as i32
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SymbolMetadata {
    pub slug: String,
    pub name: String,
    pub is_decompiled: bool,
    pub len: i32,
    pub source_id: i64,
    pub source_name: String,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub project_id: i64,
    pub project_name: String,
    pub project_repo: Option<String>,
    pub platform: i32,
}

impl SymbolMetadata {
    pub fn from_db_symbol(symbol: &DBSymbol) -> Self {
        Self {
            slug: symbol.slug.clone(),
            name: symbol.name.clone(),
            is_decompiled: symbol.is_decompiled,
            len: symbol.get_num_insns(),
            source_id: symbol.source_id,
            source_name: symbol.source_name.clone(),
            version_id: symbol.version_id,
            version_name: symbol.version_name.clone(),
            project_id: symbol.project_id,
            project_name: symbol.project_name.clone(),
            project_repo: symbol.project_repo.clone(),
            platform: symbol.platform,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DBWindow {
    pub query_start: i32,
    pub match_start: i32,
    pub len: i64,
    pub symbol_id: i64,
    pub symbol_slug: String,
    pub symbol_name: String,
    pub symbol_is_decompiled: bool,
    pub symbol_len: i32,
    pub object_symbol_idx: i32,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub source_id: i64,
    pub source_name: String,
    pub object_id: i64,
    pub object_path: String,
    pub project_id: i64,
    pub project_name: String,
    pub project_repo: Option<String>,
    pub platform: i32,
}

pub struct DBWindowResults {
    pub windows: Vec<DBWindow>,
    pub total_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmatchResult {
    pub symbol: SymbolMetadata,
    pub query_start: i64,
    pub match_start: i64,
    pub len: i64,
}

impl SubmatchResult {
    pub fn from_db_window(window: &DBWindow) -> Self {
        let platform: Platform = window.platform.try_into().expect("Unexpected platform ID");
        let num_insns = window.symbol_len / platform.arch().standard_insn_length() as i32;
        Self {
            symbol: SymbolMetadata {
                slug: window.symbol_slug.clone(),
                name: window.symbol_name.clone(),
                is_decompiled: window.symbol_is_decompiled,
                len: num_insns,
                source_id: window.source_id,
                source_name: window.source_name.clone(),
                version_id: window.version_id,
                version_name: window.version_name.clone(),
                project_id: window.project_id,
                project_name: window.project_name.clone(),
                project_repo: window.project_repo.clone(),
                platform: window.platform,
            },
            query_start: window.query_start as i64,
            match_start: window.match_start as i64,
            len: window.len,
        }
    }
}

pub async fn init() -> Result<PgPool> {
    let db_path = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    if !Postgres::database_exists(&db_path).await.unwrap_or(false) {
        match Postgres::create_database(&db_path).await {
            Ok(_) => {
                println!("Database created at {db_path}");
            }
            Err(_) => {
                return Err(anyhow::anyhow!("Error creating database"));
            }
        }
    }

    let pool = PgPool::connect(&db_path).await?;

    let migration_results = sqlx::migrate!("./migrations").run(&pool).await;

    match migration_results {
        Ok(_) => Ok(pool),
        Err(e) => Err(anyhow::anyhow!("Error migrating database: {}", e)),
    }
}

pub async fn create_version(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    platform: i32,
    project_id: i64,
) -> Result<i64> {
    match sqlx::query!(
        "INSERT INTO versions (name, platform, project_id) VALUES ($1, $2, $3) RETURNING id",
        name,
        platform,
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

pub async fn get_versions_for_project(
    conn: Pool<Postgres>,
    project_id: i64,
) -> Result<Vec<Version>> {
    let rows = sqlx::query_as!(
        Version,
        "SELECT * FROM versions WHERE versions.project_id = $1",
        project_id
    )
    .fetch_all(&conn)
    .await?;

    Ok(rows)
}

pub async fn count_versions(conn: Pool<Postgres>) -> Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM versions")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}

pub async fn create_source(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    source_link: &Option<String>,
    object_id: i64,
    version_id: Option<i64>,
    project_id: i64,
) -> Result<i64> {
    match sqlx::query!(
        "INSERT INTO sources (name, source_link, object_id, version_id, project_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id",
        name,
        *source_link,
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

pub async fn count_sources(conn: Pool<Postgres>) -> Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM sources")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}

pub async fn create_symbol_window_hashes(
    tx: &mut Transaction<'_, Postgres>,
    hashes: &[u64],
    symbol_id: i64,
) -> Result<()> {
    let hashes_enumerated: Vec<(usize, &u64)> = hashes.iter().enumerate().collect();

    for chunk in hashes_enumerated.chunks(CHUNK_SIZE) {
        let symbol_ids = vec![symbol_id; chunk.len()];
        let (poses, opcode_hashes): (Vec<i64>, Vec<i64>) =
            chunk.iter().map(|c| (c.0 as i64, *c.1 as i64)).collect();

        let r = sqlx::query!(
            "
                INSERT INTO windows (pos, hash, symbol_id)
                SELECT * FROM UNNEST($1::int[], $2::bigint[], $3::bigint[])
        ",
            &poses as &[i64],
            &opcode_hashes as &[i64],
            &symbol_ids as &[i64],
        )
        .execute(&mut **tx)
        .await;

        if let Err(e) = r {
            return Err(anyhow::anyhow!("Error adding symbol window hashes: {}", e));
        }
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmatchResultOrder {
    Length,
    QueryStart,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

pub struct QueryWindowsRequest {
    pub symbol_id: i64,
    pub start: i32,
    pub end: i32,
    pub window_size: i64,
    pub db_window_size: i64,
    pub limit: i64,
    pub page: i64,
    pub sort_by: SubmatchResultOrder,
    pub sort_direction: SortDirection,
}

pub async fn query_windows_by_symbol_id(
    conn: Pool<Postgres>,
    request: QueryWindowsRequest,
) -> Result<DBWindowResults> {
    let min_seq_len = request.window_size - request.db_window_size;
    let offset = request.page * request.limit;

    let _sort_by = match request.sort_by {
        SubmatchResultOrder::Length => "length",
        SubmatchResultOrder::QueryStart => "start_query_pos",
    };

    let _sort_dir = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };

    let rows = sqlx::query!(
        "
WITH
potential_matches AS (
    SELECT
        b.symbol_id,
        a.pos AS query_pos,
        b.pos AS match_pos,
        a.hash,
        (a.pos - b.pos) AS pos_diff
    FROM windows a
    JOIN windows b ON a.hash = b.hash
    WHERE a.pos >= $5 AND a.pos <= $6 AND a.symbol_id = $1 AND a.symbol_id != b.symbol_id
),
sequence_groups AS (
    SELECT
        hash,
        symbol_id,
        query_pos,
        match_pos,
        pos_diff,
        query_pos - ROW_NUMBER() OVER (PARTITION BY symbol_id, pos_diff ORDER BY query_pos) AS sequence_id
    FROM potential_matches
),
final_sequences AS (
    SELECT
        symbol_id,
        MIN(query_pos) AS start_query_pos,
        MIN(match_pos) AS start_match_pos,
        COUNT(*) AS length
    FROM sequence_groups
    GROUP BY symbol_id, pos_diff, sequence_id
    HAVING COUNT(*) >= $2
),
joined_sequences AS (
    SELECT
        sources.project_id,
        projects.name AS project_name,
        source_id,
        sources.name AS source_name,
        fs.symbol_id,
        symbols.name AS symbol_name,
        symbols.is_decompiled,
        symbols.slug AS symbol_slug,
        symbols.len AS symbol_len,
        symbols.symbol_idx AS object_symbol_idx,
        versions.id AS \"version_id?\",
        versions.name AS \"version_name?\",
        versions.platform,
        projects.repo AS project_repo,
        objects.id AS object_id,
        objects.local_path AS object_path,
        fs.start_query_pos,
        fs.start_match_pos,
        fs.length,
        COUNT(*) OVER() AS total_count
    FROM final_sequences fs
    JOIN symbols ON fs.symbol_id = symbols.id
    JOIN sources ON symbols.source_id = sources.id
    JOIN objects ON sources.object_id = objects.id
    JOIN versions ON sources.version_id = versions.id
    JOIN projects ON sources.project_id = projects.id
)
SELECT *
FROM joined_sequences
ORDER BY length DESC, project_id, source_id, symbol_id, start_query_pos, start_match_pos
LIMIT $3 OFFSET $4
",request.symbol_id, min_seq_len, request.limit, offset, request.start, request.end
    )
    .fetch_all(&conn)
    .await?;

    let windows: Vec<DBWindow> = rows
        .iter()
        .map(|row| DBWindow {
            query_start: row.start_query_pos.unwrap(),
            match_start: row.start_match_pos.unwrap(),
            len: row.length.unwrap() + request.db_window_size - 1,
            symbol_id: row.symbol_id,
            symbol_slug: row.symbol_slug.clone(),
            symbol_name: row.symbol_name.clone(),
            symbol_is_decompiled: row.is_decompiled,
            symbol_len: row.symbol_len,
            object_symbol_idx: row.object_symbol_idx,
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            object_id: row.object_id,
            object_path: row.object_path.clone(),
            version_id: row.version_id,
            version_name: row.version_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
            project_repo: row.project_repo.clone(),
            platform: row.platform,
        })
        .collect();

    let total_count = rows.first().map_or(0, |row| row.total_count.unwrap_or(0));

    Ok(DBWindowResults {
        windows,
        total_count,
    })
}

pub async fn count_windows(conn: Pool<Postgres>) -> anyhow::Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM windows")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}
