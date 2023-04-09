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
}

/// Global config
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config {
    db_url: "test.db".to_string(),
    testing_mode: true,
    element_pool: "pool".to_string(),
    input_folder: "res".to_string(),
});
