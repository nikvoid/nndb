use serde::Deserialize;

#[derive(Deserialize)]
pub struct StaticFolder {
    /// URL Path to folder (must include trailing slash)
    pub url: String,
    /// Physical path to folder
    pub path: String,
    /// Serve folder from nndb
    pub serve: bool,
}

#[derive(Deserialize, Clone)]
pub struct PixivCreds {
    /// Pixiv refresh token
    pub refresh_token: String,
    /// Pixiv client id
    pub client_id: String,
    /// Pixiv client secret
    pub client_secret: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")] 
pub enum ReadFiles {
    Parallel,
    Sequential,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")] 
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace
}

impl From<LogLevel> for tracing::level_filters::LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off   => Self::OFF,
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn  => Self::WARN,
            LogLevel::Info  => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

/// Application config
#[derive(Deserialize)]
pub struct Config {
    /// Url to database
    pub db_url: String,
    /// Testing mode: copy files from input to element pool instead of deleting
    pub testing_mode: bool,
    /// If true, files in input_folder will be scanned periodically
    pub auto_scan_files: bool,
    /// Set max log level
    pub log_level: LogLevel,
    /// Directory where renamed element files will be placed.
    pub element_pool: StaticFolder,
    /// Directory that will be scanned to find new element files
    pub input_folder: String,
    /// Serve thumbnails from this folder
    pub thumbnails_folder: StaticFolder,
    /// IP address to bind server to
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Folder with miscellaneous static files  
    pub static_folder: StaticFolder,
    /// File to write logs
    pub log_file: String,
    /// Pixiv fetcher credentials
    pub pixiv_credentials: Option<PixivCreds>,
    /// Path to ffmpeg.
    /// Required to generate thumbnails for animation
    pub ffmpeg_path: Option<String>,
    /// How to read files:
    /// - sequential: use one thread,
    /// - parallel: use multiple threads.
    pub read_files: ReadFiles,
    /// Max number of files stored in memory at the same time.
    /// Files data are read first to memory, then hashed and freed.
    /// Bigger values can speed up file scanning, but may use more memory.
    pub max_files_in_memory: u32,
}
