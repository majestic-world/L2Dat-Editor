use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::editor::{Options, SOURCE_KEY, atomic_write};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub chronicle: String,
    pub encryption: String,
    pub formatter: bool,
    pub enums: bool,
    pub recent: Vec<PathBuf>,
    pub width: f32,
    pub height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            chronicle: String::new(),
            encryption: SOURCE_KEY.to_owned(),
            formatter: true,
            enums: true,
            recent: Vec::new(),
            width: 1100.0,
            height: 700.0,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("Invalid settings: {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        atomic_write(&config_path()?, &serde_json::to_vec_pretty(self)?)
    }

    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|entry| entry != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(12);
    }

    pub fn options(&self) -> Options {
        Options {
            chronicle: self.chronicle.clone(),
            encryption: self.encryption.clone(),
            formatter: self.formatter,
            enums: self.enums,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("No per-user configuration directory available")?;
    Ok(base.join("L2DatEditorRust").join("settings.json"))
}
