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
    Vec<i64>,
);

#[derive(Deserialize)]
pub struct QuerySymbolsByNameRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct QuerySymbolsBySlugRequest {
    pub slug: String,
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
        let (offsets, lens, names, symbol_idxes, opcode_hashes, equiv_hashes, exact_hashes): BulkSymbolData =
            chunk
                .iter()
                .map(|s| {
                    (
                        s.offset as i64,
                        s.bytes.len() as i64,
                        s.name.clone(),
                        s.symbol_idx as i64,
                        s.opcode_hash as i64,
                        s.equiv_hash as i64,
                        s.exact_hash as i64,
                    )
                })
                .collect();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (pos, len, name, symbol_idx, opcode_hash, equiv_hash, exact_hash, source_id)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::bigint[], $5::bigint[], $6::bigint[], $7::bigint[], $8::bigint[])
                RETURNING id
        ",
            &offsets as &[i64],
            &lens as &[i64],
            &names,
            &symbol_idxes,
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
    let sym = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           symbols.symbol_idx,
           sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id AS project_id, projects.platform
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    LEFT JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.id = $1",
        query
    )
    .fetch_optional(&conn)
    .await?;

    Ok(sym)
}

pub async fn query_by_slug(conn: Pool<Postgres>, query: &str) -> anyhow::Result<Option<DBSymbol>> {
    let sym = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id AS project_id, projects.platform
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    LEFT JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.slug = $1",
        query
    )
    .fetch_optional(&conn)
    .await?;

    Ok(sym)
}

pub async fn query_by_name(
    conn: Pool<Postgres>,
    query: &QuerySymbolsByNameRequest,
) -> anyhow::Result<Vec<DBSymbol>> {
    let sym = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           sources.name AS source_name,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
           projects.name AS project_name, projects.id AS project_id, projects.platform
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    LEFT JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE symbols.name ILIKE $1",
        query.name
    )
    .fetch_all(&conn)
    .await?;

    Ok(sym)
}

pub async fn query_by_opcode_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let syms = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id, projects.platform
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

    Ok(syms)
}

pub async fn query_by_equiv_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let syms = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, 
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id, projects.platform
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

    Ok(syms)
}

pub async fn query_by_exact_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let syms = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.pos, symbols.len, symbols.name,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\",
            projects.name AS project_name, projects.id as project_id, projects.platform
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

    Ok(syms)
}
