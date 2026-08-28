use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    dry_run: bool,
) -> Result<()> {
    validate_component(profile, "profile")?;
    let source = absolute_path(source)?;
    let sources = collect_import_sources(&source, selected_names)?;
    let destination = store.root().join(profile);
    preflight_destination(sources.keys(), &destination)?;
    if dry_run {
        for (name, source) in &sources {
            println!(
                "copy\t{}\t{}",
                source.display(),
                destination.join(name).display()
            );
        }
        return Ok(());
    }
    copy_batch(&sources, &destination)
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
