use std::{fs, path::PathBuf};

use anyhow::Context;
use confique::{Config as Confique, Partial};
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3000";

#[cfg(not(debug_assertions))]
const DEFAULT_SERVER_URL: &str = "https://secrets.msmoiz.com";

/// The CLI config.
#[derive(Debug, Default, Serialize, Deserialize, Confique)]
pub struct Config {
    /// The URL for the server.
    #[config(env = "SESAME_URL")]
    pub url: String,
    /// The password for the server.
    #[config(env = "SESAME_PASSWORD")]
    pub password: Option<String>,
}

impl Config {
    /// Loads config from various sources.
    ///
    /// Values are sourced in order of priority from: environment variables and
    /// default values. Values found in a higher priority source override those
    /// found in a lower priority source. Returns an error if loading fails.
    pub fn load() -> anyhow::Result<Self> {
        Self::builder()
            .env()
            .file(config_path()?)
            .preloaded(fallback())
            .load()
            .map_err(Into::into)
    }

    /// Persists the config to file.
    pub fn store(&self) -> anyhow::Result<()> {
        let path = config_path()?;
        let parent = path.parent().expect("config path should have a parent");

        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;

        let raw = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, raw)
            .with_context(|| format!("failed to write config to {}", path.display()))?;

        Ok(())
    }
}

type ConfigLayer = <Config as Confique>::Partial;

/// Returns default values for config settings.
fn fallback() -> ConfigLayer {
    ConfigLayer {
        url: Some(DEFAULT_SERVER_URL.to_owned()),
        ..ConfigLayer::default_values()
    }
}

/// Returns the path to the CLI config file.
fn config_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(".sesame").join("config.toml"))
}
