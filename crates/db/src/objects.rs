use anyhow::Result;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use sqlx::{Pool, Postgres, Transaction};

pub async fn create(tx: &mut Transaction<'_, Postgres>, bytes: &[u8]) -> Result<i64> {
    let hash = blake3::hash(bytes);

    let bin_path = std::env::var("BIN_PATH").expect("BIN_PATH must be set");
    let target_path = Path::new(&bin_path);
    let target_path = target_path.join(format!("{hash}.bin"));

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
                    // write bytes to target_path

                    let mut file = File::create(&target_path)
                        .map_err(|e| anyhow::anyhow!("Error creating file: {}", e))?;
                    file.write_all(bytes)
                        .map_err(|e| anyhow::anyhow!("Error writing to file: {}", e))?;

                    Ok(r.id)
                } else {
                    Ok(r.id)
                }
            }
            None => Err(anyhow::anyhow!("Object not found after insert.")),
        },
        Err(e) => Err(e),
    }
}

pub async fn query_many(conn: Pool<Postgres>, hashes: &[String]) -> Result<Vec<String>> {
    let res = sqlx::query!("SELECT hash FROM objects WHERE hash = ANY($1)", hashes)
        .fetch_all(&conn)
        .await?;
    Ok(res.iter().map(|r| r.hash.clone()).collect())
}
