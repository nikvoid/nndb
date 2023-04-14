use once_cell::sync::Lazy;

/// Application config
pub struct Config {
    /// Url to database
    pub db_url: String,
    /// Testing mode: copy files from input to element pool instead of deleting
    pub testing_mode: bool,
    /// Directory where renamed element files will be placed
    pub element_pool: String,
    /// Directory that will be scanned to find new element files
    pub input_folder: String,
    /// URL Path to static files (must include trailing slash)
    pub static_files_path: String,
    /// URL Path to elements
    pub elements_path: String,
    /// IP address to bind server to
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// If Some, serve stati files from this folder
    pub static_folder: Option<String>,
}

/// Global config
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config {
    db_url: "test.db".to_string(),
    testing_mode: true,
    element_pool: "pool".to_string(),
    input_folder: "res".to_string(),
    static_files_path: "/static/".to_string(),
    elements_path: "/pool/".to_string(),
    bind_address: "0.0.0.0".to_string(),
    port: 8080,
    static_folder: Some("static".to_string()),
});
