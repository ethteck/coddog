pub mod projects;
pub mod symbols;

use anyhow::Result;
use serde::Serialize;
use sqlx::{PgPool, Pool, Postgres, Transaction, migrate::MigrateDatabase};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::{fs, fs::File, io::Read};

const CHUNK_SIZE: usize = 100000;

#[derive(Clone, Debug, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub platform: i32,
    pub repo: Option<String>,
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
    pub pos: i64,
    pub len: i32,
    pub name: String,
    pub opcode_hash: i64,
    pub equiv_hash: i64,
    pub exact_hash: i64,
    pub source_id: i64,
    pub source_name: String,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub project_id: i64,
    pub project_name: String,
}

impl Display for DBSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} version {} (offset {:X})",
            self.project_name, self.source_name, self.pos,
        ))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SymbolMetadata {
    pub slug: String,
    pub name: String,
    pub source_id: i64,
    pub source_name: String,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub project_id: i64,
    pub project_name: String,
}

impl SymbolMetadata {
    pub fn from_db_symbol(symbol: &DBSymbol) -> Self {
        Self {
            slug: symbol.slug.clone(),
            name: symbol.name.clone(),
            source_id: symbol.source_id,
            source_name: symbol.source_name.clone(),
            version_id: symbol.version_id,
            version_name: symbol.version_name.clone(),
            project_id: symbol.project_id,
            project_name: symbol.project_name.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DBWindow {
    pub query_start: i32,
    pub match_start: i32,
    pub length: i64,
    pub symbol_id: i64,
    pub symbol_slug: String,
    pub symbol_name: String,
    pub version_id: Option<i64>,
    pub version_name: Option<String>,
    pub source_id: i64,
    pub source_name: String,
    pub project_id: i64,
    pub project_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmatchResult {
    pub symbol: SymbolMetadata,
    pub query_start: i64,
    pub match_start: i64,
    pub length: i64,
}

impl SubmatchResult {
    pub fn from_db_window(window: &DBWindow) -> Self {
        Self {
            symbol: SymbolMetadata {
                slug: window.symbol_slug.clone(),
                name: window.symbol_name.clone(),
                source_id: window.source_id,
                source_name: window.source_name.clone(),
                version_id: window.version_id,
                version_name: window.version_name.clone(),
                project_id: window.project_id,
                project_name: window.project_name.clone(),
            },
            query_start: window.query_start as i64,
            match_start: window.match_start as i64,
            length: window.length,
        }
    }
}

pub async fn init() -> Result<PgPool> {
    let db_path = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    if !Postgres::database_exists(&db_path).await.unwrap_or(false) {
        match Postgres::create_database(&db_path).await {
            Ok(_) => {
                println!("Database created at {}", db_path);
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
    project_id: i64,
) -> Result<i64> {
    match sqlx::query!(
        "INSERT INTO versions (name, project_id) VALUES ($1, $2) RETURNING id",
        name,
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

pub async fn create_object(tx: &mut Transaction<'_, Postgres>, filepath: &PathBuf) -> Result<i64> {
    let mut file = File::open(filepath)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let hash = blake3::hash(&buffer);

    let bin_path = std::env::var("BIN_PATH").expect("BIN_PATH must be set");
    let target_path = Path::new(&bin_path);
    let target_path = target_path.join(format!("{}.bin", hash));

    let hash_str = hash.to_hex().to_string();

    match sqlx::query!(
        "INSERT INTO objects (hash, local_path) VALUES ($1, $2) ON CONFLICT (hash) DO NOTHING",
        &hash_str,
        target_path.to_str().unwrap(),
    )
    .execute(&mut **tx)
    .await
    .map_err(anyhow::Error::from)
    {
        Ok(_) => {}
        Err(e) => return Err(e),
    };

    match sqlx::query!("SELECT id FROM objects WHERE hash = $1", &hash_str,)
        .fetch_optional(&mut **tx)
        .await
        .map_err(anyhow::Error::from)
    {
        Ok(r) => match r {
            Some(r) => {
                if !target_path.exists() {
                    fs::create_dir_all(target_path.parent().unwrap())?;
                    match fs::copy(filepath, target_path.clone()) {
                        Ok(_) => Ok(r.id),
                        Err(e) => Err(anyhow::anyhow!("Error copying file: {}", e)),
                    }
                } else {
                    Ok(r.id)
                }
            }
            None => Err(anyhow::anyhow!("Object not found after insert.")),
        },
        Err(e) => Err(e),
    }
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
        "INSERT INTO sources (name, source_link, object_id, version_id, project_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
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
        Ok(r) => {
            Ok(r.id)
        }
        Err(e) => Err(e),
    }
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
pub async fn query_windows_by_symbol_id(
    conn: Pool<Postgres>,
    symbol_id: i64,
    window_size: i64,
    db_window_size: i64,
    limit: i64,
    page: i64,
) -> Result<Vec<DBWindow>> {
    let min_seq_len = window_size - db_window_size;

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
    WHERE a.symbol_id = $1 AND a.symbol_id != b.symbol_id
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
)
SELECT sources.project_id, projects.name AS project_name, source_id, sources.name AS source_name, 
       symbol_id, symbols.name as symbol_name, symbols.slug AS symbol_slug,
        versions.id AS \"version_id?\", versions.name AS \"version_name?\",
       start_query_pos, start_match_pos, length
FROM final_sequences
JOIN symbols ON symbol_id = symbols.id
JOIN sources ON symbols.source_id = sources.id
JOIN versions ON sources.version_id = versions.id
JOIN projects ON sources.project_id = projects.id
WHERE length >= $2
ORDER BY length DESC, project_id, source_id, symbol_id, start_query_pos, start_match_pos
LIMIT $3 OFFSET $4
",symbol_id, min_seq_len, limit, page * limit
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<DBWindow> = rows
        .iter()
        .map(|row| DBWindow {
            query_start: row.start_query_pos.unwrap(),
            match_start: row.start_match_pos.unwrap(),
            length: row.length.unwrap() + db_window_size - 1,
            symbol_id: row.symbol_id,
            symbol_slug: row.symbol_slug.clone(),
            symbol_name: row.symbol_name.clone(),
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            version_id: row.version_id,
            version_name: row.version_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}
