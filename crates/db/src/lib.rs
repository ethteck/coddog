use anyhow::Result;
use coddog_core::{Platform, Symbol};
use sqlx::{migrate::MigrateDatabase, PgPool, Pool, Postgres, Transaction};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::{fs, fs::File, io::Read};

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

    let migration_results = sqlx::migrate!("./migrations").run(&pool).await;

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
        Err(e) => Err(e),
    }
}

type BulkSymbolData = (Vec<i64>, Vec<String>, Vec<i64>, Vec<i64>, Vec<i64>);

pub async fn add_symbols(
    tx: &mut Transaction<'_, Postgres>,
    source_id: i64,
    symbols: &[Symbol],
) -> Vec<i64> {
    let mut ret = vec![];

    for chunk in symbols.chunks(CHUNK_SIZE) {
        let source_ids = vec![source_id; chunk.len()];
        let (offsets, names, opcode_hashes, equiv_hashes, exact_hashes): BulkSymbolData = chunk
            .iter()
            .map(|s| {
                (
                    s.offset as i64,
                    s.name.clone(),
                    s.opcode_hash as i64,
                    s.equiv_hash as i64,
                    s.exact_hash as i64,
                )
            })
            .collect();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (source_id, pos, name, opcode_hash, equiv_hash, exact_hash)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::bigint[], $5::bigint[], $6::bigint[])
                RETURNING id
        ",
            &source_ids as &[i64],
            &offsets as &[i64],
            &names,
            &opcode_hashes,
            &equiv_hashes,
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

#[derive(Clone, Debug)]
pub struct DBSymbol {
    pub id: i64,
    pub pos: i64,
    pub name: String,
    pub opcode_hash: i64,
    pub equiv_hash: i64,
    pub exact_hash: i64,
    pub source_id: i64,
    pub source_name: String,
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

#[derive(Clone, Debug)]
pub struct DBProject {
    pub id: i64,
    pub name: String,
}

impl Display for DBProject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.name))
    }
}

pub async fn db_query_projects_by_name(
    conn: Pool<Postgres>,
    query: &str,
) -> Result<Vec<DBProject>> {
    let rows = sqlx::query!(
        "SELECT projects.name, projects.id FROM projects WHERE projects.name LIKE $1",
        query
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<DBProject> = rows
        .iter()
        .map(|row| DBProject {
            id: row.id,
            name: query.to_string(),
        })
        .collect();

    Ok(res)
}

pub async fn db_delete_project(conn: Pool<Postgres>, id: i64) -> Result<()> {
    sqlx::query!("DELETE FROM projects WHERE id = $1", id)
        .execute(&conn)
        .await?;

    Ok(())
}

pub async fn db_query_symbols_by_name(conn: Pool<Postgres>, query: &str) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, 
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name, projects.name AS project_name, projects.id as project_id
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
            pos: row.pos,
            name: query.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}

pub async fn db_query_symbols_by_opcode_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.name, 
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.opcode_hash = $1 AND NOT symbols.id = $2",
        symbol.opcode_hash as i64,
        symbol.id as i64
    )
    .fetch_all(&conn)
    .await?;

    let res = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            pos: row.pos,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}

pub async fn db_query_symbols_by_equiv_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.name, 
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.equiv_hash = $1 AND NOT symbols.id = $2",
        symbol.equiv_hash as i64,
        symbol.id as i64
    )
    .fetch_all(&conn)
    .await?;

    let res = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            pos: row.pos,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}

pub async fn db_query_symbols_by_exact_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name, projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.exact_hash = $1 AND NOT symbols.id = $2",
        symbol.exact_hash as i64,
        symbol.id as i64
    )
    .fetch_all(&conn)
    .await?;

    let res = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            pos: row.pos,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
            source_id: row.source_id,
            source_name: row.source_name.clone(),
            project_id: row.project_id,
            project_name: row.project_name.clone(),
        })
        .collect();

    Ok(res)
}

#[derive(Clone, Debug)]
pub struct DBWindow {
    pub query_start: i32,
    pub match_start: i32,
    pub length: i64,
    pub symbol_id: i64,
    pub symbol_name: String,
    pub source_id: i64,
    pub source_name: String,
    pub project_id: i64,
    pub project_name: String,
}
pub async fn db_query_windows_by_symbol_id(
    conn: Pool<Postgres>,
    symbol_id: i64,
    min_seq_len: i64,
) -> Result<Vec<DBWindow>> {
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
SELECT project_id, projects.name AS project_name, source_id, sources.name AS source_name, symbol_id,
       symbols.name as symbol_name, start_query_pos, start_match_pos, length
FROM final_sequences
JOIN symbols ON symbol_id = symbols.id
JOIN sources ON symbols.source_id = sources.id
JOIN projects ON sources.project_id = projects.id
WHERE length >= $2
ORDER BY project_id, source_id, symbol_id, start_query_pos, start_match_pos
",symbol_id, min_seq_len
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<DBWindow> = rows
        .iter()
        .map(|row| DBWindow {
            query_start: row.start_query_pos.unwrap(),
            match_start: row.start_match_pos.unwrap(),
            length: row.length.unwrap(),
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
