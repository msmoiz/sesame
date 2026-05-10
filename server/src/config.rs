use confique::Config as Confique;

/// Server configuration.
#[derive(Confique)]
pub struct Config {
    /// The address for the HTTP server.
    #[config(env = "SESAME_ADDRESS", default = "127.0.0.1:3000")]
    pub address: String,
    /// The local path to the database file.
    #[config(env = "SESAME_DB_PATH", default = "sesame.db")]
    pub db_path: String,
    /// The password for API authentication.
    #[config(env = "SESAME_PASSWORD")]
    pub password: String,
    /// The interval between persistence flushes, in seconds.
    #[config(env = "SESAME_FLUSH_INTERVAL_SECS", default = 30)]
    pub flush_interval_secs: u64,
}

impl Config {
    /// Loads config from various sources.
    ///
    /// Values are sourced in order of priority from: environment variables and
    /// default values. Values found in a higher priority source override those
    /// found in a lower priority source. Returns an error if loading fails.
    pub fn load() -> anyhow::Result<Self> {
        Self::builder().env().load().map_err(|e| e.into())
    }
}
