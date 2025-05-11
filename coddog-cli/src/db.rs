use crate::*;
use sqlx::{migrate::MigrateDatabase, PgPool, Pool, Postgres, Transaction};
use std::{fs::File, io::Read, panic};

const CHUNK_SIZE: usize = 100000;

pub async fn db_init() -> Result<PgPool> {
    let db_path = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    if !Postgres::database_exists(&db_path).await.unwrap_or(false) {
        match Postgres::create_database(&db_path).await {
            Ok(_) => {
                println!("Database created at {}", db_path);
            }
            Err(_) => {
                panic!("Error creating database");
            }
        }
    }

    let pool = PgPool::connect(&db_path).await?;

    let migration_results = sqlx::migrate!("../migrations").run(&pool).await;

    match migration_results {
        Ok(_) => {
            println!("Database migrated");
            Ok(pool)
        }
        Err(e) => {
            panic!("Error migrating database: {}", e);
        }
    }
}

pub async fn add_project(conn: Pool<Postgres>, name: &str, platform: Platform) -> Result<i64> {
    let rec = sqlx::query!(
        "INSERT INTO projects (name, platform) VALUES ($1, $2) RETURNING id",
        name,
        platform as i32
    )
    .fetch_one(&conn)
    .await?;

    Ok(rec.id)
}

pub async fn add_source(
    conn: Pool<Postgres>,
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
    .fetch_one(&conn)
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

pub async fn add_symbols(conn: Pool<Postgres>, source_id: i64, symbols: &[Symbol]) -> Vec<i64> {
    let mut ret = vec![];

    for chunk in symbols.chunks(CHUNK_SIZE) {
        let source_ids = vec![source_id; chunk.len()];
        let (offsets, names): (Vec<i64>, Vec<String>) = chunk
            .iter()
            .map(|s| (s.offset as i64, s.name.clone()))
            .unzip();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (source_id, \"offset\", name)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[])
                RETURNING id
        ",
            &source_ids as &[i64],
            &offsets as &[i64],
            &names,
        )
        .fetch_all(&conn)
        .await
        .unwrap();

        for row in rows {
            ret.push(row.id);
        }
    }

    ret
}

pub async fn add_symbol_hashes(
    tx: &mut Transaction<'_, Postgres>,
    symbol_id: i64,
    hashes: &[u64],
) -> Result<()> {
    let hashes_enumerated: Vec<(usize, &u64)> = hashes.iter().enumerate().collect();

    for chunk in hashes_enumerated.chunks(CHUNK_SIZE) {
        let symbol_ids = vec![symbol_id; chunk.len()];
        let (offsets, hashes): (Vec<i64>, Vec<i64>) =
            chunk.iter().map(|c| (c.0 as i64, *c.1 as i64)).collect();

        let r = sqlx::query!(
            "
                INSERT INTO hashes (symbol_id, hash, \"offset\")
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::int[])
        ",
            &symbol_ids as &[i64],
            &hashes as &[i64],
            &offsets as &[i64],
        )
        .execute(&mut **tx)
        .await;

        if let Err(e) = r {
            return Err(anyhow::anyhow!("Error adding symbol hashes: {}", e));
        }
    }
    Ok(())
}
