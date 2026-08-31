use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::util::{process_is_alive, sorted_entries};

struct StateLock {
    path: PathBuf,
}

pub struct TargetLock {
    _lock: StateLock,
}

pub struct InventoryLock {
    _lock: StateLock,
}

impl TargetLock {
    pub fn acquire(target_state: &Path) -> Result<Self> {
        Ok(Self {
            _lock: StateLock::acquire(target_state, "target")?,
        })
    }
}

impl InventoryLock {
    pub fn acquire(inventory_state: &Path) -> Result<Self> {
        Ok(Self {
            _lock: StateLock::acquire(inventory_state, "inventory")?,
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

pub fn remove_legacy_activation_state(state_root: &Path, dry_run: bool) -> Result<()> {
    let targets = state_root.join("targets");
    if !targets.exists() {
        return Ok(());
    }
    let mut obsolete = Vec::new();
    for target in sorted_entries(&targets)? {
        let enabled = target.path().join("enabled");
        if fs::symlink_metadata(&enabled).is_ok() {
            obsolete.push(enabled);
        }
    }
    if obsolete.is_empty() {
        return Ok(());
    }
    if dry_run {
        for path in obsolete {
            println!("remove\t{}", path.display());
        }
        return Ok(());
    }
    for path in obsolete {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove obsolete state {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove obsolete state {}", path.display()))?;
        }
    }
    eprintln!("sm: removed obsolete per-target activation state");
    Ok(())
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

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
