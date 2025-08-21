//SELECT slug, target_assembly_id, platform, name FROM coreapp_scratch WHERE score = 0 AND max_score > 0

use sqlx::{Pool, Postgres};

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct DecompMeScratch {
    pub slug: String,
    pub target_assembly_id: String,
    pub platform: String,
    pub name: String,
    pub diff_label: String,
}
pub async fn query_all_matched_scratches(
    conn: Pool<Postgres>,
) -> anyhow::Result<Vec<DecompMeScratch>> {
    let scratches = sqlx::query_as(
        "SELECT slug, target_assembly_id, platform, name, diff_label
         FROM coreapp_scratch
         WHERE score = 0 AND max_score > 0",
    )
    .fetch_all(&conn)
    .await?;

    Ok(scratches)
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct AssemblyRow {
    pub elf_object: Vec<u8>,
    pub source_asm_id: Option<String>,
}

pub async fn query_scratch_assembly(
    conn: Pool<Postgres>,
    slug: &str,
) -> anyhow::Result<AssemblyRow> {
    let obj = sqlx::query_as::<_, AssemblyRow>(
        "SELECT elf_object, source_asm_id
         FROM coreapp_scratch
         JOIN coreapp_assembly ON coreapp_assembly.hash = coreapp_scratch.target_assembly_id
         WHERE slug = $1",
    )
    .bind(slug)
    .fetch_one(&conn)
    .await?;

    Ok(obj)
}
