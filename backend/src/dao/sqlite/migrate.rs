use std::collections::HashMap;

use anyhow::bail;
use sqlx::{SqlitePool, SqliteConnection, migrate::{Migrate, MigrationType}};
use tracing::info;
use crate::CONFIG;

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
                    run_proc(proc, &mut tx).await?;
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

async fn run_proc(name: &str, tx: &mut SqliteConnection) -> anyhow::Result<()> {
match name {
    "add_file_time" => {
        let files: Vec<String> = sqlx::query_scalar!(
            "SELECT filename FROM element"  
        )
        .fetch_all(&mut *tx)
        .await?;

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
        
        Ok(())
    }    
        
    _ => bail!("no such procedure: `{}`", name)
}
} 
