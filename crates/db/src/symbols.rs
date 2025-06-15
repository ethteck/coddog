use crate::{CHUNK_SIZE, DBSymbol};
use coddog_core::Symbol;
use serde::Deserialize;
use sqlx::{Pool, Postgres, Transaction};

type BulkSymbolData = (
    Vec<i64>,
    Vec<i64>,
    Vec<String>,
    Vec<i64>,
    Vec<i64>,
    Vec<i64>,
);

#[derive(Deserialize)]
pub struct QuerySymbolsByNameRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct QueryWindowsRequest {
    pub slug: String,
    pub min_length: i64,
    pub page: i64,
    pub size: i64,
}

pub async fn create(
    tx: &mut Transaction<'_, Postgres>,
    source_id: i64,
    symbols: &[Symbol],
) -> Vec<i64> {
    let mut ret = vec![];

    for chunk in symbols.chunks(CHUNK_SIZE) {
        let source_ids = vec![source_id; chunk.len()];
        let (offsets, lens, names, opcode_hashes, equiv_hashes, exact_hashes): BulkSymbolData =
            chunk
                .iter()
                .map(|s| {
                    (
                        s.offset as i64,
                        s.length as i64,
                        s.name.clone(),
                        s.opcode_hash as i64,
                        s.equiv_hash as i64,
                        s.exact_hash as i64,
                    )
                })
                .collect();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (pos, len, name, opcode_hash, equiv_hash, exact_hash, source_id)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::bigint[], $5::bigint[], $6::bigint[], $7::bigint[])
                RETURNING id
        ",
            &offsets as &[i64],
            &lens as &[i64],
            &names,
            &opcode_hashes,
            &equiv_hashes,
            &exact_hashes,
            &source_ids as &[i64],
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

pub async fn query_by_id(conn: Pool<Postgres>, query: i64) -> anyhow::Result<Option<DBSymbol>> {
    let row = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id as project_id
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    LEFT JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.id = $1",
        query
    )
    .fetch_optional(&conn)
    .await?;

    let res = row.map(|row| DBSymbol {
        id: row.id,
        slug: row.slug.clone(),
        pos: row.pos,
        len: row.len,
        name: row.name.to_string(),
        opcode_hash: row.opcode_hash,
        equiv_hash: row.equiv_hash,
        exact_hash: row.exact_hash,
        source_id: row.source_id,
        source_name: row.source_name.clone(),
        version_id: row.version_id,
        version_name: row.version_name.clone(),
        project_id: row.project_id,
        project_name: row.project_name.clone(),
    });

    Ok(res)
}

pub async fn query_by_slug(conn: Pool<Postgres>, query: &str) -> anyhow::Result<Option<DBSymbol>> {
    let row = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id as project_id
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.slug = $1",
        query
    )
    .fetch_optional(&conn)
    .await?;

    let res = row.map(|row| DBSymbol {
        id: row.id,
        slug: row.slug.clone(),
        pos: row.pos,
        len: row.len,
        name: row.name.to_string(),
        opcode_hash: row.opcode_hash,
        equiv_hash: row.equiv_hash,
        exact_hash: row.exact_hash,
        source_id: row.source_id,
        source_name: row.source_name.clone(),
        version_id: row.version_id,
        version_name: row.version_name.clone(),
        project_id: row.project_id,
        project_name: row.project_name.clone(),
    });

    Ok(res)
}

pub async fn query_by_name(
    conn: Pool<Postgres>,
    query: &QuerySymbolsByNameRequest,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id as project_id
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.name ILIKE $1",
        query.name
    )
    .fetch_all(&conn)
    .await?;

    let res: Vec<DBSymbol> = rows
        .iter()
        .map(|row| DBSymbol {
            id: row.id,
            slug: row.slug.clone(),
            pos: row.pos,
            len: row.len,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
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

pub async fn query_by_opcode_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN versions ON versions.id = sources.version_id
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
            slug: row.slug.clone(),
            pos: row.pos,
            len: row.len,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
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

pub async fn query_by_equiv_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN versions ON versions.id = sources.version_id
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
            slug: row.slug.clone(),
            pos: row.pos,
            len: row.len,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
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

pub async fn query_by_exact_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN versions ON versions.id = sources.version_id
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
            slug: row.slug.clone(),
            pos: row.pos,
            len: row.len,
            name: row.name.to_string(),
            opcode_hash: row.opcode_hash,
            equiv_hash: row.equiv_hash,
            exact_hash: row.exact_hash,
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
