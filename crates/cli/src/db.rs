use crate::{DbCommands, MatchType, get_full_path};
use anyhow::{Result, anyhow};
use coddog_core::Platform;
use coddog_core::ingest::read_elf;
use coddog_db::projects::CreateProjectRequest;
use coddog_db::symbols::QuerySymbolsByNameRequest;
use coddog_db::{DBSymbol, DBWindow, QueryWindowsRequest};
use decomp_settings::read_config;
use glob::glob;
use inquire::Select;
use itertools::Itertools;
use pbr::ProgressBar;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
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
        let res = Select::new("Which project do you want to check?", projects).prompt();
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
                    platform: platform as i32,
                    repo: config.repo.clone(),
                },
            )
            .await?;

            let mut tx = pool.begin().await?;

            for version in &config.versions {
                let version_id =
                    coddog_db::create_version(&mut tx, &version.fullname, project_id).await?;

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
                    let object_id = coddog_db::create_object(&mut tx, &obj_file).await?;
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
                            coddog_db::symbols::create(&mut tx, source_id, &symbols).await;

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
    }
    Ok(())
}
