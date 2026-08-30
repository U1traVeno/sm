use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::profiles::ProfileStore;
use crate::util::{absolute_path, sorted_entries, symlink_dir, validate_component};

pub fn export_skills(
    selected: &BTreeMap<String, PathBuf>,
    destination: &Path,
    dry_run: bool,
) -> Result<()> {
    let destination = absolute_path(destination)?;
    preflight_destination(selected.keys(), &destination)?;
    if dry_run {
        for (name, source) in selected {
            println!(
                "copy\t{}\t{}",
                source.display(),
                destination.join(name).display()
            );
        }
        return Ok(());
    }
    copy_batch(selected, &destination)
}

pub fn import_skills(
    store: &ProfileStore,
    profile: &str,
    source: &Path,
    selected_names: &[String],
    replace: bool,
    dry_run: bool,
) -> Result<()> {
    validate_component(profile, "profile")?;
    let source = absolute_path(source)?;
    let sources = collect_import_sources(&source, selected_names)?;
    let destination = store.root().join(profile);
    preflight_import_destination(sources.keys(), &destination, replace)?;
    if dry_run {
        for (name, source) in &sources {
            let destination = destination.join(name);
            println!(
                "{}\t{}\t{}",
                if entry_exists(&destination)? {
                    "replace"
                } else {
                    "copy"
                },
                source.display(),
                destination.display()
            );
        }
        return Ok(());
    }
    if replace {
        replace_batch(&sources, &destination)
    } else {
        copy_batch(&sources, &destination)
    }
}

fn collect_import_sources(
    source: &Path,
    selected_names: &[String],
) -> Result<BTreeMap<String, PathBuf>> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("import source does not exist: {}", source.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("import source is not a directory: {}", source.display());
    }
    let mut available = BTreeMap::new();
    for entry in sorted_entries(source)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        validate_component(&name, "skill")?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!(
                "import entry is not a directory: {}",
                entry.path().display()
            );
        }
        available.insert(name, entry.path());
    }
    if selected_names.is_empty() {
        return Ok(available);
    }
    let mut selected = BTreeMap::new();
    for name in selected_names {
        validate_component(name, "skill")?;
        let path = available
            .get(name)
            .with_context(|| format!("import skill does not exist: {name}"))?;
        selected.insert(name.clone(), path.clone());
    }
    Ok(selected)
}

fn preflight_destination<'a>(
    names: impl Iterator<Item = &'a String>,
    destination: &Path,
) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(destination)
        && (!metadata.is_dir() || metadata.file_type().is_symlink())
    {
        bail!(
            "copy destination is not a directory: {}",
            destination.display()
        );
    }
    for name in names {
        let path = destination.join(name);
        if fs::symlink_metadata(&path).is_ok() {
            bail!("copy destination already exists: {}", path.display());
        }
    }
    Ok(())
}

fn preflight_import_destination<'a>(
    names: impl Iterator<Item = &'a String>,
    destination: &Path,
    replace: bool,
) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(destination)
        && (!metadata.is_dir() || metadata.file_type().is_symlink())
    {
        bail!(
            "copy destination is not a directory: {}",
            destination.display()
        );
    }
    for name in names {
        let path = destination.join(name);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to inspect import destination {}", path.display())
                });
            }
        };
        if !replace {
            bail!("copy destination already exists: {}", path.display());
        }
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!("replace destination is not a directory: {}", path.display());
        }
    }
    Ok(())
}

fn entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to inspect {}", path.display())),
    }
}

fn replace_batch(sources: &BTreeMap<String, PathBuf>, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let transaction = format!("{}.{}", std::process::id(), nonce);
    let staging = destination.join(format!(".sm-import-{transaction}"));
    let backup = destination.join(format!(".sm-import-backup-{transaction}"));
    if entry_exists(&staging)? || entry_exists(&backup)? {
        bail!(
            "import transaction path already exists in {}",
            destination.display()
        );
    }

    fs::create_dir(&staging)?;
    let staged = (|| -> Result<()> {
        for (name, source) in sources {
            copy_tree(source, &staging.join(name))?;
        }
        Ok(())
    })();
    if let Err(error) = staged {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    fs::create_dir(&backup)?;
    let mut installed = Vec::new();
    let mut backed_up = Vec::new();
    let committed = (|| -> Result<()> {
        for name in sources.keys() {
            let final_path = destination.join(name);
            if entry_exists(&final_path)? {
                fs::rename(&final_path, backup.join(name))
                    .with_context(|| format!("failed to back up imported skill {name}"))?;
                backed_up.push(name.clone());
            }
            fs::rename(staging.join(name), &final_path)
                .with_context(|| format!("failed to install imported skill {name}"))?;
            installed.push(name.clone());
        }
        Ok(())
    })();

    if let Err(error) = committed {
        let rollback = rollback_import(destination, &backup, &installed, &backed_up);
        let _ = fs::remove_dir_all(&staging);
        if let Err(rollback_error) = rollback {
            return Err(error.context(format!(
                "import rollback failed: {rollback_error:#}; backup remains at {}",
                backup.display()
            )));
        }
        let _ = fs::remove_dir(&backup);
        return Err(error);
    }

    fs::remove_dir(&staging)?;
    fs::remove_dir_all(&backup)
        .with_context(|| format!("failed to remove import backup {}", backup.display()))
}

fn rollback_import(
    destination: &Path,
    backup: &Path,
    installed: &[String],
    backed_up: &[String],
) -> Result<()> {
    let mut failures = Vec::new();
    for name in installed.iter().rev() {
        let path = destination.join(name);
        if let Err(error) = fs::remove_dir_all(&path) {
            failures.push(format!("failed to remove {}: {error}", path.display()));
        }
    }
    for name in backed_up.iter().rev() {
        let source = backup.join(name);
        let destination = destination.join(name);
        if let Err(error) = fs::rename(&source, &destination) {
            failures.push(format!(
                "failed to restore {}: {error}",
                destination.display()
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("{}", failures.join("; "))
    }
}

fn copy_batch(sources: &BTreeMap<String, PathBuf>, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    let staging = destination.join(format!(".sm-copy-{}", std::process::id()));
    if staging.exists() {
        bail!(
            "copy staging directory already exists: {}",
            staging.display()
        );
    }
    fs::create_dir(&staging)?;
    let result = (|| -> Result<()> {
        for (name, source) in sources {
            copy_tree(source, &staging.join(name))?;
        }
        for name in sources.keys() {
            fs::rename(staging.join(name), destination.join(name))
                .with_context(|| format!("failed to install copied skill {name}"))?;
        }
        fs::remove_dir(&staging)?;
        Ok(())
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(source)?;
        symlink_dir(&target, destination)?;
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in sorted_entries(source)? {
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        fs::set_permissions(destination, metadata.permissions())?;
        return Ok(());
    }
    if metadata.is_file() {
        fs::copy(source, destination).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        fs::set_permissions(destination, metadata.permissions())?;
        return Ok(());
    }
    bail!("unsupported file type in skill: {}", source.display())
}
