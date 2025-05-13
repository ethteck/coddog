use crate::*;
use sqlx::{migrate::MigrateDatabase, PgPool, Pool, Postgres, Transaction};
use std::fmt::{Display, Formatter};
use std::{fs::File, io::Read};

const CHUNK_SIZE: usize = 100000;

pub async fn db_init() -> Result<PgPool> {
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

    let migration_results = sqlx::migrate!("../migrations").run(&pool).await;

    match migration_results {
        Ok(_) => Ok(pool),
        Err(e) => Err(anyhow::anyhow!("Error migrating database: {}", e)),
    }
}

pub async fn add_project(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    platform: Platform,
) -> Result<i64> {
    let rec = sqlx::query!(
        "INSERT INTO projects (name, platform) VALUES ($1, $2) RETURNING id",
        name,
        platform as i32
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(rec.id)
}

pub async fn add_source(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    name: &str,
    filepath: &PathBuf,
) -> Result<i64> {
    let mut file = File::open(filepath)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let hash = blake3::hash(&buffer);

    let bin_path = std::env::var("BIN_PATH").expect("BIN_PATH must be set");
    let target_path = Path::new(&bin_path);
    let target_path = target_path.join(format!("{}.bin", hash));

    match sqlx::query!(
        "INSERT INTO sources (project_id, hash, name, filepath) VALUES ($1, $2, $3, $4) RETURNING id",
        project_id,
        &hash.to_hex().to_string(),
        name,
        target_path.to_str().unwrap(),
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(anyhow::Error::from)
    {
        Ok(r) => {
            fs::create_dir_all(target_path.parent().unwrap())?; // Ensure the target directory exists
            match fs::copy(filepath, target_path.clone()) {
                Ok(_) => Ok(r.id),
                Err(e) => Err(anyhow::anyhow!("Error copying file: {}", e)),
            }
        }
        Err(e) => Err(anyhow::anyhow!("Error adding source to database: {}", e)),
    }
}

pub async fn add_symbols(
    tx: &mut Transaction<'_, Postgres>,
    source_id: i64,
    symbols: &[Symbol],
) -> Vec<i64> {
    let mut ret = vec![];

    for chunk in symbols.chunks(CHUNK_SIZE) {
        let source_ids = vec![source_id; chunk.len()];
        let (offsets, names, fuzzy_hashes, exact_hashes): (
            Vec<i64>,
            Vec<String>,
            Vec<i64>,
            Vec<i64>,
        ) = chunk
            .iter()
            .map(|s| {
                (
                    s.offset as i64,
                    s.name.clone(),
                    s.fuzzy_hash as i64,
                    s.exact_hash as i64,
                )
            })
            .collect();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (source_id, pos, name, fuzzy_hash, exact_hash)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::bigint[], $5::bigint[])
                RETURNING id
        ",
            &source_ids as &[i64],
            &offsets as &[i64],
            &names,
            &fuzzy_hashes,
            &exact_hashes,
        )
        .fetch_all(&mut **tx)
        .await
        .unwrap();

        for row in rows {
            ret.push(row.id);
        }
    }

    ret
}

pub async fn add_symbol_window_hashes(
    tx: &mut Transaction<'_, Postgres>,
    symbol_id: i64,
    hashes: &[u64],
) -> Result<()> {
    let hashes_enumerated: Vec<(usize, &u64)> = hashes.iter().enumerate().collect();

    for chunk in hashes_enumerated.chunks(CHUNK_SIZE) {
        let symbol_ids = vec![symbol_id; chunk.len()];
        let (poses, fuzzy_hashes): (Vec<i64>, Vec<i64>) =
            chunk.iter().map(|c| (c.0 as i64, *c.1 as i64)).collect();

        let r = sqlx::query!(
            "
                INSERT INTO windows (symbol_id, pos, hash)
                SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::bigint[])
        ",
            &symbol_ids as &[i64],
            &poses as &[i64],
            &fuzzy_hashes as &[i64],
        )
        .execute(&mut **tx)
        .await;

        if let Err(e) = r {
            return Err(anyhow::anyhow!("Error adding symbol window hashes: {}", e));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct DBSymbol {
    pub id: i64,
    pub source_id: i64,
    pub pos: i64,
    pub name: String,
    pub project: String,
    pub version: String,
    pub fuzzy_hash: i64,
    pub exact_hash: i64,
}

impl Display for DBSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} version {} (offset {:X})",
            self.project, self.version, self.pos,
        ))
    }
}

pub async fn db_query_symbols_by_name(conn: Pool<Postgres>, query: &str) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "SELECT symbols.id, symbols.source_id, symbols.pos, symbols.fuzzy_hash, symbols.exact_hash,
        projects.name AS project, sources.name AS version
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN projects on sources.project_id = projects.id
                                             WHERE symbols.name = $1",
        query
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<DBSymbol> = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            source_id: row.source_id,
            pos: row.pos,
            name: query.to_string(),
            fuzzy_hash: row.fuzzy_hash,
            exact_hash: row.exact_hash,
            project: row.project.clone(),
            version: row.version.clone(),
        })
        .collect();

    Ok(res)
}

pub async fn db_query_symbols_by_fuzzy_hash(
    conn: Pool<Postgres>,
    hash: i64,
) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
SELECT symbols.id, symbols.source_id, symbols.pos, symbols.name, symbols.fuzzy_hash, 
       symbols.exact_hash, sources.name AS version, projects.name AS project
FROM symbols
INNER JOIN sources ON sources.id = symbols.source_id
INNER JOIN projects on sources.project_id = projects.id
WHERE symbols.fuzzy_hash = $1",
        hash
    )
    .fetch_all(&conn)
    .await?;

    let res = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            source_id: row.source_id,
            pos: row.pos,
            name: row.name.to_string(),
            fuzzy_hash: row.fuzzy_hash,
            exact_hash: row.exact_hash,
            project: row.project.clone(),
            version: row.version.clone(),
        })
        .collect();

    Ok(res)
}

pub async fn db_query_windows_by_symbol_id_fuzzy(
    conn: Pool<Postgres>,
    id: i64,
) -> Result<Vec<i64>> {
    let rows = sqlx::query!("SELECT windows.hash FROM windows WHERE symbol_id = $1", id)
        .fetch_all(&conn)
        .await?;

    let res: Vec<i64> = rows.iter().map(|row| row.hash).collect();

    Ok(res)
}

pub struct HashResult {
    pub id: i64,
    pub pos: i32,
    pub symbol_id: i64,
    pub symbol_name: String,
    pub source_id: i64,
    pub source_name: String,
    pub project_id: i64,
    pub project_name: String,
}
pub async fn db_query_windows_by_symbol_hashes_fuzzy(
    conn: Pool<Postgres>,
    hashes: &[i64],
    symbol_id: i64,
) -> Result<Vec<HashResult>> {
    let rows = sqlx::query!(
        "
    SELECT windows.id AS hash_id, symbols.id AS symbol_id, source_id, windows.pos,
           symbols.name AS symbol_name, sources.name AS source_name, projects.id AS project_id,
           projects.name AS project_name
    FROM windows
    INNER JOIN symbols ON symbols.id = windows.symbol_id
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN projects on projects.id = sources.project_id
    WHERE windows.hash = ANY($1) AND NOT symbols.id = $2
    ",
        hashes,
        symbol_id,
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<HashResult> = rows
        .iter()
        .map(|row| HashResult {
            id: row.hash_id,
            pos: row.pos,
            symbol_id: row.symbol_id,
            symbol_name: row.symbol_name.clone(),
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}
