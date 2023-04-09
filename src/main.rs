use tracing::info;

use crate::service::scan_files;

mod model;
mod dao;
mod import;
mod service;
mod config; 



fn main() {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .init();
    info!("Starting service");
    scan_files().unwrap();
}
