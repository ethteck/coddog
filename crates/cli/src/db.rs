use crate::{DbCommands, MatchType, get_full_path};
use anyhow::{Result, anyhow};
use coddog_core::ingest::read_elf;
use coddog_core::{Platform, Symbol};
use coddog_db::decompme::DecompMeScratch;
use coddog_db::projects::CreateProjectRequest;
use coddog_db::symbols::QuerySymbolsByNameRequest;
use coddog_db::{DBSymbol, DBWindow, QueryWindowsRequest, SortDirection, SubmatchResultOrder};
use decomp_settings::read_config;
use glob::glob;
use inquire::Select;
use itertools::Itertools;
use pbr::ProgressBar;
use sqlx::{PgPool, Pool, Postgres};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

async fn db_search_symbol_by_name(conn: Pool<Postgres>, name: &str) -> anyhow::Result<DBSymbol> {
    let symbols = coddog_db::symbols::query_by_name(
        conn,
        &QuerySymbolsByNameRequest {
            name: name.to_string(),
        },
    )
    .await?;

    if symbols.is_empty() {
        return Err(anyhow!("No symbols found with the name '{}'", name));
    }

    if symbols.len() > 1 {
        let res = Select::new("Which symbol do you want to check?", symbols).prompt();
        Ok(res?)
    } else {
        Ok(symbols.first().unwrap().clone())
    }
}

async fn db_search_project_by_name(conn: Pool<Postgres>, name: &str) -> anyhow::Result<i64> {
    let projects = coddog_db::projects::query_by_name(conn, name).await?;

    if projects.is_empty() {
        return Err(anyhow!("No projects found with the name '{}'", name));
    }

    if projects.len() > 1 {
        let res = Select::new("Which project do you want to select?", projects).prompt();
        Ok(res?.id)
    } else {
        Ok(projects.first().unwrap().id)
    }
}

struct SubmatchResults {
    projects: Vec<SubmatchProjectResults>,
}

impl SubmatchResults {
    fn from_db_hashes(
        hashes: &[DBWindow],
        project_map: &mut HashMap<i64, String>,
        source_map: &mut HashMap<i64, String>,
        symbol_map: &mut HashMap<i64, String>,
    ) -> Self {
        let mut results = SubmatchResults { projects: vec![] };

        for (project_id, project_rows) in &hashes.iter().chunk_by(|h| h.project_id) {
            let project_rows = project_rows.collect_vec();
            let project_name = &project_rows.first().unwrap().project_name;
            project_map.insert(project_id, project_name.to_string());

            let mut project_results = SubmatchProjectResults {
                id: project_id,
                sources: vec![],
            };

            for (source_id, source_rows) in &project_rows.iter().chunk_by(|h| h.source_id) {
                let source_rows = source_rows.collect_vec();
                let source_name = &source_rows.first().unwrap().source_name;
                source_map.insert(source_id, source_name.to_string());

                let mut source_results = SubmatchObjectResults {
                    id: source_id,
                    symbols: vec![],
                };

                for (symbol_id, symbol_rows) in &source_rows.into_iter().chunk_by(|h| h.symbol_id) {
                    let symbol_rows = symbol_rows.collect_vec();
                    let sym_name = &symbol_rows.first().unwrap().symbol_name;
                    symbol_map.insert(symbol_id, sym_name.clone());

                    let sym_results = SubmatchSymbolResults {
                        id: symbol_id,
                        slices: symbol_rows
                            .into_iter()
                            .map(|h| SubmatchSliceResults {
                                query_start: h.query_start,
                                match_start: h.match_start,
                                length: h.len,
                            })
                            .collect(),
                    };
                    source_results.symbols.push(sym_results);
                }
                project_results.sources.push(source_results);
            }
            results.projects.push(project_results);
        }

        results
    }

    fn to_string(
        &self,
        window_size: usize,
        project_map: &HashMap<i64, String>,
        source_map: &HashMap<i64, String>,
        symbol_map: &HashMap<i64, String>,
    ) -> String {
        let mut result = String::new();
        for project in &self.projects {
            result.push_str(&format!("{}:\n", project_map.get(&project.id).unwrap()));
            for object in &project.sources {
                result.push_str(&format!(
                    "\tVersion {}:\n",
                    source_map.get(&object.id).unwrap()
                ));
                for symbol in &object.symbols {
                    result.push_str(&format!("\t\t{}:\n", symbol_map.get(&symbol.id).unwrap()));
                    for slice in &symbol.slices {
                        result.push_str(&format!(
                            "\t\t\t[{}/{}] ({} insns)\n",
                            slice.query_start,
                            slice.match_start,
                            slice.length as usize + window_size - 1
                        ));
                    }
                }
            }
        }
        result
    }
}

struct SubmatchProjectResults {
    id: i64,
    sources: Vec<SubmatchObjectResults>,
}

struct SubmatchObjectResults {
    id: i64,
    symbols: Vec<SubmatchSymbolResults>,
}

struct SubmatchSymbolResults {
    id: i64,
    slices: Vec<SubmatchSliceResults>,
}

struct SubmatchSliceResults {
    query_start: i32,
    match_start: i32,
    length: i64,
}

pub(crate) async fn handle_db_command(cmd: &DbCommands) -> Result<()> {
    match cmd {
        DbCommands::AddProject { repo } => {
            let yaml = repo.join("decomp.yaml");
            let config = read_config(yaml.clone())?;
            let platform = Platform::from_name(&config.platform).unwrap();
            let window_size = std::env::var("DB_WINDOW_SIZE")
                .expect("DB_WINDOW_SIZE must be set")
                .parse::<usize>()?;

            let pool = coddog_db::init().await?;

            let project_id = coddog_db::projects::create(
                pool.clone(),
                &CreateProjectRequest {
                    name: config.name.clone(),
                    repo: config.repo.clone(),
                },
            )
            .await?;

            let mut tx = pool.begin().await?;

            for version in &config.versions {
                let version_id = coddog_db::create_version(
                    &mut tx,
                    &version.fullname,
                    platform as i32,
                    project_id,
                )
                .await?;

                let obj_files: Vec<PathBuf> = glob(&format!(
                    "{}/**/*.o",
                    get_full_path(
                        yaml.parent().unwrap(),
                        Some(version.paths.build_dir.clone())
                    )
                    .unwrap()
                    .to_str()
                    .unwrap()
                ))?
                .filter_map(Result::ok)
                .collect();

                let mut pb = ProgressBar::new(obj_files.len() as u64);
                pb.format("[=>-]");
                pb.message(format!("Importing objects ({}) ", version.fullname).as_str());

                for obj_file in obj_files {
                    pb.inc();
                    let obj_bytes = std::fs::read(&obj_file)?;
                    let object_id = coddog_db::objects::create(&mut tx, &obj_bytes).await?;
                    let source_id = coddog_db::create_source(
                        &mut tx,
                        obj_file.file_name().unwrap().to_str().unwrap(),
                        &config.repo,
                        object_id,
                        Option::from(version_id),
                        project_id,
                    )
                    .await?;

                    let obj_bytes = std::fs::read(&obj_file)?;
                    let symbols = read_elf(platform, &None, &obj_bytes)?;

                    if !symbols.is_empty() {
                        let symbol_ids =
                            coddog_db::symbols::create_many(&mut tx, source_id, &symbols).await;

                        for (symbol, id) in symbols.iter().zip(symbol_ids) {
                            let opcode_hashes = symbol.get_opcode_hashes(window_size);

                            coddog_db::create_symbol_window_hashes(&mut tx, &opcode_hashes, id)
                                .await?;
                        }
                    }
                }
                println!();
            }
            tx.commit().await?;
            println!("Imported project {} ", config.name);
        }
        DbCommands::DeleteProject { name } => {
            let pool = coddog_db::init().await?;

            let project = db_search_project_by_name(pool.clone(), name).await?;

            coddog_db::projects::delete(pool.clone(), project).await?;
            println!("Deleted project {name}");
        }
        DbCommands::CleanBins {} => {
            let bin_path = std::env::var("BIN_PATH").expect("BIN_PATH must be set");

            let bins: Vec<PathBuf> = glob(&format!("{}/*.bin", bin_path))?
                .filter_map(Result::ok)
                .collect();

            const CHUNK_SIZE: usize = 1000;

            let mut deleted_bins = 0;

            let pool = coddog_db::init().await?;
            let mut pb = ProgressBar::new(bins.len() as u64);
            pb.format("[=>-]");
            pb.message("Processing bins ");
            for bin_chunk in &bins.iter().chunks(CHUNK_SIZE) {
                let chunk_bins = bin_chunk.cloned().collect_vec();
                let on_disk = chunk_bins
                    .iter()
                    .map(|p| p.file_stem().unwrap().to_str().unwrap().to_string())
                    .collect_vec();

                let in_db = coddog_db::objects::query_many(pool.clone(), &on_disk).await?;

                let to_delete = HashSet::<String>::from_iter(on_disk)
                    .difference(&HashSet::<String>::from_iter(in_db))
                    .cloned()
                    .collect_vec();

                for hash in to_delete {
                    let path = PathBuf::from(format!("{}/{}.bin", bin_path, hash));
                    if path.exists() {
                        std::fs::remove_file(&path)?;
                        deleted_bins += 1;
                    }
                }
            }
            println!("Removed {} bins", deleted_bins);
        }
        DbCommands::Match { query, match_type } => {
            let pool = coddog_db::init().await?;

            let symbol = db_search_symbol_by_name(pool.clone(), query).await?;

            let matches = match match_type {
                MatchType::Opcode => {
                    coddog_db::symbols::query_by_opcode_hash(pool.clone(), &symbol).await?
                }
                MatchType::Equivalent => {
                    coddog_db::symbols::query_by_equiv_hash(pool.clone(), &symbol).await?
                }
                MatchType::Exact => {
                    coddog_db::symbols::query_by_exact_hash(pool.clone(), &symbol).await?
                }
            };

            if matches.is_empty() {
                println!("No matches found");
            } else {
                for sym in matches {
                    println!("{} - {} {}", sym.name, sym.project_name, sym.source_name);
                }
            }
        }
        DbCommands::Submatch { query, window_size } => {
            let db_window_size = std::env::var("DB_WINDOW_SIZE")
                .expect("DB_WINDOW_SIZE must be set")
                .parse::<usize>()?;

            if *window_size < db_window_size {
                return Err(anyhow!("Window size must be at least {}", db_window_size));
            }

            let pool = coddog_db::init().await?;

            let symbol = db_search_symbol_by_name(pool.clone(), query).await?;

            let before_time = SystemTime::now();
            let matching_hashes = coddog_db::query_windows_by_symbol_id(
                pool.clone(),
                QueryWindowsRequest {
                    symbol_id: symbol.id,
                    start: 0,
                    end: symbol.get_num_insns(),
                    window_size: *window_size as i64,
                    db_window_size: db_window_size as i64,
                    limit: 100,
                    page: 0,
                    sort_by: SubmatchResultOrder::Length,
                    sort_direction: SortDirection::Desc,
                },
            )
            .await?;

            match before_time.elapsed() {
                Ok(elapsed) => {
                    println!("Big query took {}ms", elapsed.as_millis());
                }
                Err(e) => {
                    println!("Error: {e:?}");
                }
            }

            if matching_hashes.windows.is_empty() {
                println!("No matches found");
                return Ok(());
            }

            let mut project_map: HashMap<i64, String> = HashMap::new();
            let mut source_map: HashMap<i64, String> = HashMap::new();
            let mut symbol_map: HashMap<i64, String> = HashMap::new();

            let results = SubmatchResults::from_db_hashes(
                &matching_hashes.windows,
                &mut project_map,
                &mut source_map,
                &mut symbol_map,
            );

            println!(
                "{}",
                results.to_string(*window_size, &project_map, &source_map, &symbol_map)
            );
        }
        DbCommands::ImportDecompme {} => {
            let decompme_db_url = std::env::var("DECOMPME_DATABASE_URL")
                .expect("DECOMPME_DATABASE_URL must be set")
                .parse::<String>()?;
            let window_size = std::env::var("DB_WINDOW_SIZE")
                .expect("DB_WINDOW_SIZE must be set")
                .parse::<usize>()?;
            let supported_platforms = [
                Platform::N64,
                Platform::Psx,
                Platform::Ps2,
                Platform::Psp,
                Platform::GcWii,
            ];

            let decompme_pool = PgPool::connect(&decompme_db_url).await?;

            let scratches =
                coddog_db::decompme::query_all_matched_scratches(decompme_pool.clone()).await?;
            println!("Found {} scratches", scratches.len());

            let scratches: Vec<DecompMeScratch> = scratches
                .iter()
                .filter(|scratch| {
                    let platform = Platform::from_decompme_name(&scratch.platform);

                    if platform.is_none() {
                        return false;
                    }
                    let platform = platform.unwrap();
                    supported_platforms.contains(&platform)
                })
                .cloned()
                .collect::<Vec<DecompMeScratch>>();
            println!(
                "Filtered by platform to {} supported scratches",
                scratches.len()
            );

            let pool = coddog_db::init().await?;

            let project_id = coddog_db::projects::query_by_name(pool.clone(), "decomp.me").await?;
            if project_id.is_empty() {
                return Err(anyhow!("Project 'decomp.me' not found in the database"));
            }
            if project_id.len() > 1 {
                return Err(anyhow!("Multiple projects found with the name 'decomp.me'"));
            }
            let project_id = project_id.first().unwrap().id;

            println!("Using project ID: {}", project_id);

            let versions = coddog_db::get_versions_for_project(pool.clone(), project_id).await?;

            let mut tx = pool.begin().await?;

            let mut pb = ProgressBar::new(scratches.len() as u64);
            pb.format("[=>-]");
            pb.message("Importing scratches ");

            let mut imported = 0;
            let mut asm_scratches = 0;
            let mut no_symbols = 0;
            let mut cant_find_symbol = 0;
            let mut no_bytes = 0;

            for scratch in scratches {
                pb.inc();

                let platform = Platform::from_decompme_name(&scratch.platform).unwrap();

                let version_id = versions
                    .iter()
                    .find(|v| v.platform == platform as i32)
                    .map(|v| v.id)
                    .ok_or_else(|| {
                        anyhow!(
                            "Version for platform {} not found in the database",
                            scratch.platform
                        )
                    })?;

                let elf_object = coddog_db::decompme::query_scratch_assembly(
                    decompme_pool.clone(),
                    &scratch.slug,
                )
                .await?;

                // let skips = vec!["996k9", "gDP9Y", "RiLDB", "gcxsC"];
                //
                // if skips.contains(&&*scratch.slug) {
                //     continue;
                // }

                let symbols = read_elf(platform, &None, &elf_object.elf_object);

                if let Err(e) = symbols {
                    println!("Error reading ELF for scratch {}: {}", scratch.slug, e);
                    continue;
                }
                let symbols = symbols.unwrap();

                let from_object = elf_object.source_asm_id.is_none();

                if from_object {
                    //object_scratches += 1;
                } else {
                    asm_scratches += 1;
                }

                if symbols.is_empty() {
                    // return Err(anyhow!(
                    //     "No symbols found in {} scratch {}",
                    //     scratch_type,
                    //     scratch.slug
                    // ));
                    no_symbols += 1;
                    continue;
                }

                let matched_sym = match symbols.len() {
                    1 => Some(symbols.first().unwrap()),
                    _ => symbols
                        .iter()
                        .find(|s| s.name == scratch.name || s.name == scratch.diff_label),
                };

                if matched_sym.is_none() {
                    // anyhow!(
                    //                 "No symbol found with name '{}' or '{}' in {} scratch {}",
                    //                 scratch.name,
                    //                 scratch.diff_label,
                    //                 scratch_type,
                    //                 scratch.slug
                    //             )
                    cant_find_symbol += 1;
                    continue;
                }

                let matched_sym = matched_sym.unwrap();

                if matched_sym.bytes.is_empty() {
                    // return Err(anyhow!(
                    //     "Symbol {} in {} scratch {} has no bytes",
                    //     matched_sym.name,
                    //     scratch_type,
                    //     scratch.slug
                    // ));
                    no_bytes += 1;
                    continue;
                }

                let matched_sym = Symbol {
                    is_decompiled: true,
                    ..matched_sym.clone()
                };

                let object_id = coddog_db::objects::create(&mut tx, &elf_object.elf_object).await?;

                let source_id = coddog_db::create_source(
                    &mut tx,
                    &scratch.slug,
                    &Some(format!("https://decomp.me/scratch/{}", scratch.slug)),
                    object_id,
                    Option::from(version_id),
                    project_id,
                )
                .await?;

                let symbol_id =
                    coddog_db::symbols::create_one(&mut tx, source_id, &matched_sym).await;

                let opcode_hashes = matched_sym.get_opcode_hashes(window_size);
                coddog_db::create_symbol_window_hashes(&mut tx, &opcode_hashes, symbol_id).await?;
                imported += 1;
            }

            tx.commit().await?;
            pb.finish_print("Imported scratches successfully");

            println!("Imported {} scratches", imported);
            println!("ASM scratches: {}", asm_scratches);
            println!("ASM scratches with no symbols: {}", no_symbols);
            println!("ASM scratches can't find symbol: {}", cant_find_symbol);
            println!("ASM scratches with no bytes: {}", no_bytes);
        }
    }
    Ok(())
}
