use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::util::{process_is_alive, sorted_entries, validate_component};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnabledProfile {
    pub sequence: u64,
    pub name: String,
}

struct StateLock {
    path: PathBuf,
}

pub struct TargetLock {
    _lock: StateLock,
}

pub struct ProfileLock {
    _lock: StateLock,
}

impl TargetLock {
    pub fn acquire(target_state: &Path) -> Result<Self> {
        Ok(Self {
            _lock: StateLock::acquire(target_state, "target")?,
        })
    }
}

impl ProfileLock {
    pub fn acquire(profile_state: &Path) -> Result<Self> {
        Ok(Self {
            _lock: StateLock::acquire(profile_state, "profile")?,
        })
    }
}

impl StateLock {
    fn acquire(state: &Path, kind: &str) -> Result<Self> {
        fs::create_dir_all(state)
            .with_context(|| format!("failed to create {kind} state {}", state.display()))?;
        let path = state.join("lock");
        for _ in 0..4 {
            let temporary = state.join(format!("lock.tmp.{}.{}", std::process::id(), nonce()));
            fs::create_dir(&temporary)?;
            fs::create_dir(temporary.join(format!("owner-{}", std::process::id())))?;
            match fs::rename(&temporary, &path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) => {
                    let _ = fs::remove_dir_all(&temporary);
                    if !path.exists() {
                        return Err(error).with_context(|| {
                            format!("failed to lock {kind} state {}", path.display())
                        });
                    }
                }
            }

            if lock_has_live_or_unknown_owner(&path)? {
                bail!("{kind} is locked by another sm process: {}", path.display());
            }
            let stale = state.join(format!("lock.stale.{}.{}", std::process::id(), nonce()));
            if fs::rename(&path, &stale).is_ok() {
                fs::remove_dir_all(stale)?;
            }
        }
        bail!("failed to acquire {kind} lock: {}", path.display())
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn lock_has_live_or_unknown_owner(path: &Path) -> Result<bool> {
    let entries = sorted_entries(path)?;
    if entries.is_empty() {
        return Ok(false);
    }
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix("owner-")
            .and_then(|value| value.parse::<u32>().ok())
        else {
            return Ok(true);
        };
        if process_is_alive(pid) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn load_enabled(target_state: &Path) -> Result<Vec<EnabledProfile>> {
    let path = target_state.join("enabled");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut enabled = Vec::new();
    for entry in sorted_entries(&path)? {
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!(
                "enabled marker is not a directory: {}",
                entry.path().display()
            );
        }
        let marker = entry.file_name().to_string_lossy().into_owned();
        let Some((sequence, name)) = marker.split_once('-') else {
            bail!("invalid enabled marker: {marker}");
        };
        let sequence = sequence
            .parse::<u64>()
            .with_context(|| format!("invalid enabled marker sequence: {marker}"))?;
        validate_component(name, "profile")?;
        enabled.push(EnabledProfile {
            sequence,
            name: name.to_owned(),
        });
    }
    enabled.sort_by_key(|entry| entry.sequence);
    let mut names = BTreeSet::new();
    for pair in enabled.windows(2) {
        if pair[0].sequence == pair[1].sequence {
            bail!("duplicate enabled marker sequence: {}", pair[0].sequence);
        }
    }
    for entry in &enabled {
        if !names.insert(&entry.name) {
            bail!("duplicate enabled profile marker: {}", entry.name);
        }
    }
    Ok(enabled)
}

pub fn enable_profiles(enabled: &mut Vec<EnabledProfile>, profiles: &[String]) -> Result<()> {
    let mut next = enabled
        .iter()
        .map(|entry| entry.sequence)
        .max()
        .unwrap_or(0);
    for profile in profiles {
        validate_component(profile, "profile")?;
        enabled.retain(|entry| entry.name != *profile);
        next = next
            .checked_add(1)
            .context("enabled profile sequence overflow")?;
        enabled.push(EnabledProfile {
            sequence: next,
            name: profile.clone(),
        });
    }
    Ok(())
}

pub fn disable_profiles(enabled: &mut Vec<EnabledProfile>, profiles: &[String]) -> Result<()> {
    for profile in profiles {
        validate_component(profile, "profile")?;
        enabled.retain(|entry| entry.name != *profile);
    }
    Ok(())
}

pub fn save_enabled(target_state: &Path, enabled: &[EnabledProfile]) -> Result<()> {
    fs::create_dir_all(target_state)?;
    let nonce = nonce();
    let staging = target_state.join(format!("enabled.tmp.{}.{}", std::process::id(), nonce));
    let backup = target_state.join(format!("enabled.old.{}.{}", std::process::id(), nonce));
    fs::create_dir(&staging)?;
    let result = (|| -> Result<()> {
        for entry in enabled {
            fs::create_dir(staging.join(format!("{:020}-{}", entry.sequence, entry.name)))?;
        }
        let current = target_state.join("enabled");
        let had_current = current.exists();
        if had_current {
            fs::rename(&current, &backup)?;
        }
        if let Err(error) = fs::rename(&staging, &current) {
            if had_current {
                let _ = fs::rename(&backup, &current);
            }
            return Err(error.into());
        }
        if had_current {
            fs::remove_dir_all(&backup)?;
        }
        Ok(())
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.with_context(|| format!("failed to save enabled state in {}", target_state.display()))
}

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
