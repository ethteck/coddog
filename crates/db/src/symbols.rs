use crate::{CHUNK_SIZE, DBSymbol};
use coddog_core::Symbol;
use serde::Deserialize;
use sqlx::{Pool, Postgres, Transaction};

type BulkSymbolData = (
    Vec<i64>,
    Vec<String>,
    Vec<bool>,
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

pub async fn create_many(
    tx: &mut Transaction<'_, Postgres>,
    source_id: i64,
    symbols: &[Symbol],
) -> Vec<i64> {
    let mut ret = vec![];

    for chunk in symbols.chunks(CHUNK_SIZE) {
        let source_ids = vec![source_id; chunk.len()];
        let (
            lens,
            names,
            is_decompileds,
            symbol_idxes,
            opcode_hashes,
            equiv_hashes,
            exact_hashes,
        ): BulkSymbolData = chunk
            .iter()
            .map(|s| {
                (
                    s.bytes.len() as i64,
                    s.name.clone(),
                    s.is_decompiled,
                    s.symbol_idx as i64,
                    s.opcode_hash as i64,
                    s.equiv_hash as i64,
                    s.exact_hash as i64,
                )
            })
            .collect();

        let rows = sqlx::query!(
            "
                INSERT INTO symbols (len, name, is_decompiled, symbol_idx, opcode_hash, equiv_hash, exact_hash, source_id)
                SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::boolean[], $4::bigint[], $5::bigint[], $6::bigint[], $7::bigint[], $8::bigint[])
                RETURNING id
        ",
            &lens as &[i64],
            &names,
            &is_decompileds,
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

pub async fn create_one(
    tx: &mut Transaction<'_, Postgres>,
    source_id: i64,
    symbol: &Symbol,
) -> i64 {
    let row = sqlx::query!(
            "
                INSERT INTO symbols (len, name, is_decompiled, symbol_idx, opcode_hash, equiv_hash, exact_hash, source_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id
        ",
        symbol.bytes.len() as i64,
        symbol.name.clone(),
        symbol.is_decompiled,
        symbol.symbol_idx as i64,
        symbol.opcode_hash as i64,
        symbol.equiv_hash as i64,
        symbol.exact_hash as i64,
        source_id
        )
        .fetch_one(&mut **tx)
        .await
        .unwrap();

    row.id
}

pub async fn query_by_id(conn: Pool<Postgres>, query: i64) -> anyhow::Result<Option<DBSymbol>> {
    let sym = sqlx::query_as!(
        DBSymbol,
        "
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
           symbols.symbol_idx,
           sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
           projects.name AS project_name, projects.id AS project_id,
           projects.repo AS project_repo
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
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
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
           projects.name AS project_name, projects.id AS project_id,
           projects.repo AS project_repo
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
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
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
           projects.name AS project_name, projects.id AS project_id,
           projects.repo AS project_repo
    FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
    LEFT JOIN versions ON versions.id = sources.version_id
    INNER JOIN projects on sources.project_id = projects.id
    WHERE strict_word_similarity (symbols.name, $1) > 0.5
    ORDER BY strict_word_similarity (symbols.name, $1) DESC",
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
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
            projects.name AS project_name, projects.id as project_id,
           projects.repo AS project_repo
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
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
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash,
           symbols.source_id,
            sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
            projects.name AS project_name, projects.id as project_id,
           projects.repo AS project_repo
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
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
    SELECT symbols.id, symbols.slug, symbols.len, symbols.name, symbols.is_decompiled,
           symbols.symbol_idx,
           symbols.opcode_hash, symbols.equiv_hash, symbols.exact_hash, symbols.source_id,
            sources.name AS source_name, objects.local_path AS object_path, symbols.symbol_idx AS object_symbol_idx,
           versions.id AS \"version_id?\", versions.name AS \"version_name?\", versions.platform,
            projects.name AS project_name, projects.id as project_id,
           projects.repo AS project_repo
        FROM symbols
    INNER JOIN sources ON sources.id = symbols.source_id
    INNER JOIN objects ON objects.id = sources.object_id
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

pub async fn count(conn: Pool<Postgres>) -> anyhow::Result<i64> {
    let rec = sqlx::query!("SELECT COUNT(*) as count FROM symbols")
        .fetch_one(&conn)
        .await?;

    Ok(rec.count.unwrap_or(0))
}
