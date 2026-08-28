use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::util::{normalize_path, sorted_entries, symlink_dir};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkOperation {
    Link {
        source: PathBuf,
        destination: PathBuf,
    },
    Unlink {
        destination: PathBuf,
    },
}

pub fn plan_links(
    desired: &BTreeMap<String, PathBuf>,
    target: &Path,
    profiles_root: &Path,
) -> Result<Vec<LinkOperation>> {
    if let Ok(metadata) = fs::symlink_metadata(target)
        && (!metadata.is_dir() || metadata.file_type().is_symlink())
    {
        bail!("persistent target is not a directory: {}", target.display());
    }

    let mut existing = BTreeMap::new();
    if target.exists() {
        for entry in sorted_entries(target)? {
            existing.insert(
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            );
        }
    }

    let mut operations = Vec::new();
    for (name, source) in desired {
        let destination = target.join(name);
        match existing.remove(name) {
            None => operations.push(LinkOperation::Link {
                source: source.clone(),
                destination,
            }),
            Some(path) => {
                let metadata = fs::symlink_metadata(&path)?;
                if !metadata.file_type().is_symlink() {
                    bail!(
                        "unmanaged target entry blocks skill {name}: {}",
                        path.display()
                    );
                }
                let current = resolved_link_target(&path)?;
                if current == normalize_path(source) {
                    continue;
                }
                if !is_managed_target(&current, profiles_root) {
                    bail!(
                        "unmanaged target link blocks skill {name}: {}",
                        path.display()
                    );
                }
                operations.push(LinkOperation::Link {
                    source: source.clone(),
                    destination,
                });
            }
        }
    }

    for path in existing.into_values() {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            let current = resolved_link_target(&path)?;
            if is_managed_target(&current, profiles_root) {
                operations.push(LinkOperation::Unlink { destination: path });
            }
        }
    }
    Ok(operations)
}

pub fn apply_links(operations: &[LinkOperation], target: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        for operation in operations {
            match operation {
                LinkOperation::Link {
                    source,
                    destination,
                } => {
                    println!("link\t{}\t{}", source.display(), destination.display());
                }
                LinkOperation::Unlink { destination } => {
                    println!("unlink\t{}", destination.display());
                }
            }
        }
        return Ok(());
    }

    fs::create_dir_all(target)
        .with_context(|| format!("failed to create target {}", target.display()))?;
    for operation in operations {
        match operation {
            LinkOperation::Link {
                source,
                destination,
            } => {
                atomic_link(source, destination)?;
            }
            LinkOperation::Unlink { destination } => {
                fs::remove_file(destination).with_context(|| {
                    format!("failed to remove managed link {}", destination.display())
                })?;
            }
        }
    }
    Ok(())
}

pub fn managed_status(
    target: &Path,
    profiles_root: &Path,
) -> Result<Vec<(String, PathBuf, PathBuf)>> {
    if !target.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entry in sorted_entries(target)? {
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.file_type().is_symlink() {
            continue;
        }
        let source = resolved_link_target(&entry.path())?;
        if is_managed_target(&source, profiles_root) {
            result.push((
                entry.file_name().to_string_lossy().into_owned(),
                source,
                entry.path(),
            ));
        }
    }
    Ok(result)
}

fn atomic_link(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .context("link destination has no parent")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".sm-link-{}.{}", std::process::id(), nonce));
    symlink_dir(source, &temporary)?;
    match fs::rename(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error).with_context(|| {
                format!(
                    "failed to install managed link {} -> {}",
                    destination.display(),
                    source.display()
                )
            })
        }
    }
}

fn resolved_link_target(link: &Path) -> Result<PathBuf> {
    let target =
        fs::read_link(link).with_context(|| format!("failed to read link {}", link.display()))?;
    let absolute = if target.is_absolute() {
        target
    } else {
        link.parent().context("link has no parent")?.join(target)
    };
    Ok(normalize_path(&absolute))
}

fn is_managed_target(target: &Path, profiles_root: &Path) -> bool {
    let target = normalize_path(target);
    let profiles_root = normalize_path(profiles_root);
    let Ok(relative) = target.strip_prefix(&profiles_root) else {
        return false;
    };
    let components = relative.components().collect::<Vec<_>>();
    components.len() == 2
        && components
            .iter()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}
