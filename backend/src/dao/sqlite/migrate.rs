use std::{collections::HashMap, ops::ControlFlow};

use anyhow::bail;
use sqlx::{SqlitePool, SqliteConnection, migrate::{Migrate, MigrationType}};
use tracing::{info, warn};
use crate::{CONFIG, import::{ElementPrefab, Parser, Fetcher}, model::read::PendingImport};

/// Run migrations with ability to call rust procedures.
///
/// This is simplified version of [sqlx::migrate::Migrator::run].
///
/// Registered rust procedures can be invoked with 
/// '''sql
/// -- RUN <rust_procedure_name> 
/// '''
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    tx.ensure_migrations_table().await?;

    let applied: HashMap<_, _> = tx.list_applied_migrations().await?
        .into_iter()
        .map(|m| (m.version, m.checksum))
        .collect();
    
    for mig in sqlx::migrate!().iter() {
        if mig.migration_type != MigrationType::Simple {
            bail!("only simple migrations are supported");
        }

        match applied.get(&mig.version) {
            Some(cksum) => if cksum != &mig.checksum {
                bail!("migration `{}` has different checksum", mig.description);
            },
            None => {
                tx.apply(mig).await?;
                for proc in get_procs(&mig.sql) {
                    info!("running procedure `{proc}` (part of `{}` migration)", mig.description);
                    if run_proc(proc, &mut tx).await?.is_break() {
                        // Rollback if Break requested
                        tx.rollback().await?;
                        std::process::exit(0);
                    };
                }
            },
        }
    }         
       
    tx.commit().await?;
    
    Ok(())
}

pub fn get_procs(sql: &str) -> Vec<&str> {
    sql.lines()
        .filter(|l| l.starts_with("-- RUN"))
        .filter_map(|l| l.split_whitespace().nth(2))
        .collect()
}

async fn run_proc(name: &str, tx: &mut SqliteConnection) -> anyhow::Result<ControlFlow<()>> {
    let files: Vec<String> = sqlx::query_scalar!(
        "SELECT filename FROM element"  
    )
    .fetch_all(&mut *tx)
    .await?;
    
    match name {
        "add_file_time" => {
            for file in files {
                let path = CONFIG.element_pool.path.join(&file);
                if let Ok(time) = crate::util::get_file_datetime(&path) {
                    sqlx::query!(
                        "UPDATE element SET file_time = ? WHERE filename = ?",
                        time, 
                        file
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        
            Ok(ControlFlow::Continue(()))
        }    

        "add_raw_sd_meta" => {
            for file in files {
                let path = CONFIG.element_pool.path.join(&file);
                let prefab = ElementPrefab {
                    data: std::fs::read(&path)?,
                    path,
                };

                let parser = Parser::scan(&prefab);
                if parser != Parser::Passthrough {
                    let meta = parser.extract_metadata(&prefab)?;
                    let raw_meta = meta.raw_meta;

                    sqlx::query!(
                        "UPDATE metadata
                        SET raw_meta = ? 
                        FROM (SELECT id FROM element WHERE filename = ?)
                        WHERE metadata.element_id = id",
                        raw_meta, 
                        file
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
            
            Ok(ControlFlow::Continue(()))
        }

        "add_raw_pixiv_meta" => {
            let imports: Vec<PendingImport> = sqlx::query_as(
                "SELECT e.*, m.importer_id
                FROM element e
                JOIN metadata m ON m.element_id = e.id
                WHERE m.importer_id = ?",
            )
            .bind(Fetcher::Pixiv)
            .fetch_all(&mut *tx)
            .await?;

            if imports.is_empty() {
                return Ok(ControlFlow::Continue(()))
            }
            
            // Ask for user decision if fetcher is not available
            if !Fetcher::Pixiv.available() {
                println!(
"
For this migration you need to fill [pixiv_credentials] section in your config file
Print:
    `ok` to stop migration and fill credentials, or
    `skip` to skip this migration and do not reimport pixiv metadata, or
    `clear` to clear imported pixiv metadata, so you can re-run import manually later 
"
                );

                let mut line = String::new();

                loop {
                    println!("input: ");
                    std::io::stdin()
                        .read_line(&mut line)?;

                    match line.trim() {
                        "ok" => return Ok(ControlFlow::Break(())),
                        "skip" => return Ok(ControlFlow::Continue(())),
                        "clear" => break,
                        _ => println!("unrecognized option, try again")
                    }
                }

                // This code can only be reached from `clear` branch
                sqlx::query!(
                    "DELETE FROM metadata WHERE importer_id = ?",
                    Fetcher::Pixiv
                )
                .execute(&mut *tx)
                .await?;
                sqlx::query!(
                    "DELETE FROM fetch_status WHERE importer_id = ?",
                    Fetcher::Pixiv
                )
                .execute(&mut *tx)
                .await?;
                return Ok(ControlFlow::Continue(()));
            }

            for (idx, import) in imports.iter().enumerate() {
                if let Some(meta) = Fetcher::Pixiv.fetch_metadata(import).await? {
                    sqlx::query!(
                        "UPDATE metadata
                        SET raw_meta = ?
                        WHERE element_id = ?
                        ",
                        meta.raw_meta,
                        import.id
                    )
                    .execute(&mut *tx)
                    .await?;
                    
                    info!("processed {}/{}", idx + 1, imports.len());
                }  else {
                    warn!(?import, "metadata was not found on server");
                }
            }
            
            Ok(ControlFlow::Continue(()))
        }
        
        _ => bail!("no such procedure: `{}`", name)
    }
} 
