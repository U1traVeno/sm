use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Table, value};

use crate::util::{absolute_path, require_absolute_config_path, validate_component};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[allow(dead_code)]
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
        }
        let mut persistent_paths = BTreeMap::<PathBuf, String>::new();
        for (name, target) in &self.targets {
            validate_component(name, "target")?;
            if let Some(path) = &target.skills_dir {
                let path =
                    require_absolute_config_path(path, &format!("targets.{name}.skills_dir"))?;
                if let Some(previous) = persistent_paths.insert(path.clone(), name.clone()) {
                    bail!(
                        "targets {previous} and {name} use the same skills directory: {}",
                        path.display()
                    );
                }
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

    pub fn persistent_targets(&self, explicit: Option<&str>) -> Result<Vec<(String, PathBuf)>> {
        if let Some(name) = explicit {
            let target = self
                .targets
                .get(name)
                .with_context(|| format!("unknown target: {name}"))?;
            return Ok(vec![(name.to_owned(), target.persistent_path(name)?)]);
        }
        let mut targets = Vec::new();
        for (name, target) in &self.targets {
            if let Some(path) = &target.skills_dir {
                targets.push((
                    name.clone(),
                    require_absolute_config_path(path, &format!("targets.{name}.skills_dir"))?,
                ));
            }
        }
        if targets.is_empty() {
            bail!("no persistent target is configured");
        }
        Ok(targets)
    }

    pub fn shell_adapters(&self, selected: &[String]) -> Result<BTreeMap<String, ShellTemplate>> {
        let names = if selected.is_empty() {
            self.targets.keys().cloned().collect::<Vec<_>>()
        } else {
            selected.to_vec()
        };
        let mut adapters = BTreeMap::new();
        for name in names {
            let target = self
                .targets
                .get(&name)
                .with_context(|| format!("unknown target: {name}"))?;
            match &target.shell {
                Some(shell) => {
                    adapters.insert(name, shell.clone());
                }
                None if !selected.is_empty() => bail!("target {name} has no shell adapter"),
                None => {}
            }
        }
        if adapters.is_empty() {
            bail!("no shell adapter is configured");
        }
        Ok(adapters)
    }

    pub fn adopt(config_path: &Path, name: &str, directory: &Path, dry_run: bool) -> Result<()> {
        validate_component(name, "target")?;
        let directory = absolute_path(directory)?;
        let existing = Self::load(config_path)?;
        for (other_name, target) in &existing.targets {
            let Some(other_path) = &target.skills_dir else {
                continue;
            };
            let other_path = require_absolute_config_path(
                other_path,
                &format!("targets.{other_name}.skills_dir"),
            )?;
            if other_name == name {
                if other_path == directory {
                    return Ok(());
                }
                bail!(
                    "target {name} already uses a different skills directory: {}",
                    other_path.display()
                );
            }
            if other_path == directory {
                bail!(
                    "skills directory is already adopted by target {other_name}: {}",
                    directory.display()
                );
            }
        }

        if dry_run {
            println!("adopt\t{name}\t{}", directory.display());
            return Ok(());
        }

        let text = match fs::read_to_string(config_path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let mut document = text
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        if !document.contains_key("targets") {
            document["targets"] = Item::Table(Table::new());
        }
        let targets = document["targets"]
            .as_table_mut()
            .context("config targets value is not a table")?;
        if !targets.contains_key(name) {
            targets[name] = Item::Table(Table::new());
        }
        let target = targets[name]
            .as_table_mut()
            .with_context(|| format!("config target {name} is not a table"))?;
        target["skills_dir"] = value(directory.to_string_lossy().as_ref());

        let rendered = document.to_string();
        let parsed: Self = toml::from_str(&rendered).with_context(|| {
            format!(
                "adopt would create invalid config {}",
                config_path.display()
            )
        })?;
        parsed.validate()?;
        let parent = config_path.parent().context("config path has no parent")?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".config.toml.sm-{}", std::process::id()));
        fs::write(&temporary, rendered)?;
        match fs::rename(&temporary, config_path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error).with_context(|| format!("failed to update {}", config_path.display()))
            }
        }
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
