use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::util::{normalize_path, user_home};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let user_home = user_home()?;
        let home = match env::var_os("SM_HOME") {
            Some(value) => absolute_env_path(Path::new(&value), "SM_HOME")?,
            None => user_home.join(".sm"),
        };
        let config_base = xdg_path("XDG_CONFIG_HOME", &user_home.join(".config"))?;
        let state_base = xdg_path("XDG_STATE_HOME", &user_home.join(".local").join("state"))?;
        let cache_base = xdg_path("XDG_CACHE_HOME", &user_home.join(".cache"))?;

        Ok(Self {
            home: normalize_path(&home),
            config: config_base.join("sm").join("config.toml"),
            state: state_base.join("sm"),
            cache: cache_base.join("sm"),
        })
    }

    pub fn profiles(&self) -> PathBuf {
        self.home.join("profiles")
    }

    pub fn target_state(&self, target: &str) -> PathBuf {
        self.state.join("targets").join(target)
    }

    pub fn inventory_state(&self) -> PathBuf {
        self.state.join("inventory")
    }

    pub fn generations(&self) -> PathBuf {
        self.cache.join("generations")
    }

    pub fn leases(&self) -> PathBuf {
        self.state.join("leases")
    }
}

fn xdg_path(variable: &str, fallback: &Path) -> Result<PathBuf> {
    match env::var_os(variable) {
        Some(value) if !value.is_empty() => absolute_env_path(Path::new(&value), variable),
        _ => Ok(fallback.to_path_buf()),
    }
}

fn absolute_env_path(path: &Path, variable: &str) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("{variable} must be an absolute path: {}", path.display());
    }
    path.canonicalize().or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(normalize_path(path))
        } else {
            Err(error).with_context(|| format!("failed to resolve {variable}"))
        }
    })
}
