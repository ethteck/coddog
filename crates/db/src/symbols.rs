use crate::{DBSymbol, CHUNK_SIZE};
use coddog_core::Symbol;
use sqlx::{Pool, Postgres, Transaction};

type BulkSymbolData = (
    Vec<i64>,
    Vec<i64>,
    Vec<String>,
    Vec<i64>,
    Vec<i64>,
    Vec<i64>,
);

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
                INSERT INTO symbols (source_id, pos, len, name, opcode_hash, equiv_hash, exact_hash)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::bigint[], $4::text[], $5::bigint[], $6::bigint[], $7::bigint[])
                RETURNING id
        ",
            &source_ids as &[i64],
            &offsets as &[i64],
            &lens as &[i64],
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

pub async fn query_by_name(conn: Pool<Postgres>, query: &str) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.len,
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
            len: row.len,
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

pub async fn query_by_opcode_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.len, symbols.name,
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
            len: row.len,
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

pub async fn query_by_equiv_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.len, symbols.name,
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
            len: row.len,
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

pub async fn query_by_exact_hash(
    conn: Pool<Postgres>,
    symbol: &DBSymbol,
) -> anyhow::Result<Vec<DBSymbol>> {
    let rows = sqlx::query!(
        "
    SELECT symbols.id, symbols.pos, symbols.len, symbols.name,
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
            len: row.len,
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
