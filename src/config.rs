use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::util::{require_absolute_config_path, validate_component};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    pub default_target: Option<String>,
    #[serde(default)]
    pub targets: BTreeMap<String, Target>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub skills_dir: Option<PathBuf>,
    pub shell: Option<ShellTemplate>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShellTemplate {
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };
        let config: Self =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(default) = &self.default_target {
            validate_component(default, "target")?;
            if !self.targets.contains_key(default) {
                bail!("default target is not configured: {default}");
            }
        }
        for (name, target) in &self.targets {
            validate_component(name, "target")?;
            if let Some(path) = &target.skills_dir {
                require_absolute_config_path(path, &format!("targets.{name}.skills_dir"))?;
            }
            if let Some(shell) = &target.shell {
                if shell.command.as_os_str().is_empty() {
                    bail!("targets.{name}.shell.command must not be empty");
                }
                let has_placeholder = shell.args.iter().any(|value| value.contains("{skills}"))
                    || shell.env.values().any(|value| value.contains("{skills}"));
                if !has_placeholder {
                    bail!("targets.{name}.shell must contain {{skills}} in args or env");
                }
                for key in shell.env.keys() {
                    if key.is_empty()
                        || !key
                            .chars()
                            .all(|character| character == '_' || character.is_ascii_alphanumeric())
                        || key.as_bytes()[0].is_ascii_digit()
                    {
                        bail!("invalid environment variable in target {name}: {key}");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn select_target<'a>(&'a self, explicit: Option<&str>) -> Result<(&'a str, &'a Target)> {
        let selected = explicit
            .map(ToOwned::to_owned)
            .or_else(|| std::env::var("SM_TARGET").ok())
            .or_else(|| self.default_target.clone())
            .or_else(|| {
                if self.targets.len() == 1 {
                    self.targets.keys().next().cloned()
                } else {
                    None
                }
            });
        let Some(name) = selected else {
            let available = self.targets.keys().cloned().collect::<Vec<_>>().join(", ");
            if available.is_empty() {
                bail!("no target is configured");
            }
            bail!("target is ambiguous; available targets: {available}");
        };
        let (stored_name, target) = self
            .targets
            .get_key_value(&name)
            .with_context(|| format!("unknown target: {name}"))?;
        Ok((stored_name.as_str(), target))
    }

    pub fn target(&self, name: &str) -> Result<&Target> {
        self.targets
            .get(name)
            .with_context(|| format!("unknown target: {name}"))
    }
}

impl Target {
    pub fn persistent_path(&self, name: &str) -> Result<PathBuf> {
        let path = self
            .skills_dir
            .as_deref()
            .with_context(|| format!("target {name} has no persistent skills_dir"))?;
        require_absolute_config_path(path, &format!("targets.{name}.skills_dir"))
    }
}
