use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::materialize::{is_managed_skill_target, resolved_link_target};
use crate::profiles::{ProfileStore, TAG_FILE};
use crate::util::{absolute_path, normalize_path, sorted_entries, symlink_dir, validate_component};

pub fn export_skills(
    selected: &BTreeMap<String, PathBuf>,
    destination: &Path,
    dry_run: bool,
) -> Result<()> {
    let destination = absolute_path(destination)?;
    preflight_export(selected.keys(), &destination)?;
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
    let items = selected
        .iter()
        .map(|(name, source)| InstallItem {
            source: source.clone(),
            destination: destination.join(name),
        })
        .collect::<Vec<_>>();
    install_batch(&items, &destination, false)
}

#[allow(clippy::too_many_arguments)]
pub fn import_skills(
    store: &ProfileStore,
    profile: &str,
    source: &Path,
    selected_names: &[String],
    create: bool,
    replace: bool,
    dry_run: bool,
) -> Result<()> {
    validate_component(profile, "profile")?;
    let source = absolute_path(source)?;
    let sources = collect_sources(&source, selected_names, store.root())?;
    let profile_exists = store.profile_exists(profile)?;
    if !profile_exists && !create {
        bail!("profile does not exist: {profile}; rerun with --create to create it");
    }
    let profile_enabled = if profile_exists {
        store.profile_enabled(profile)?
    } else {
        true
    };
    store.validate_enabled_additions(profile, profile_enabled, sources.entries.keys())?;
    let destination = store.root().join(profile);

    if profile_exists {
        preflight_import(sources.entries.keys(), &destination, replace)?;
    }
    if dry_run {
        if !profile_exists {
            println!("create-profile\ttrue\t{}", destination.display());
        }
        for (name, source) in &sources.entries {
            let target = destination.join(name);
            println!(
                "{}\t{}\t{}",
                if target.exists() { "replace" } else { "copy" },
                source.display(),
                target.display()
            );
        }
        return Ok(());
    }

    fs::create_dir_all(store.root())
        .with_context(|| format!("failed to create {}", store.root().display()))?;
    if !profile_exists {
        return create_profile_with_sources(&sources.entries, &destination);
    }
    let items = sources
        .entries
        .iter()
        .map(|(name, source)| InstallItem {
            source: source.clone(),
            destination: destination.join(name),
        })
        .collect::<Vec<_>>();
    install_batch(&items, &destination, replace)
}

pub fn update_skills(
    store: &ProfileStore,
    source: &Path,
    selected_names: &[String],
    profile: Option<&str>,
    all: bool,
    dry_run: bool,
) -> Result<()> {
    if let Some(profile) = profile
        && !store.profile_exists(profile)?
    {
        bail!("profile does not exist: {profile}");
    }
    let source = absolute_path(source)?;
    let sources = collect_sources(&source, selected_names, store.root())?;
    let mut inventory = BTreeMap::<String, Vec<(String, PathBuf)>>::new();
    for owner in store.list_profiles()? {
        for (name, path) in store.skills(&owner)? {
            inventory
                .entry(name)
                .or_default()
                .push((owner.clone(), path));
        }
    }

    let mut items = Vec::new();
    let mut skipped = Vec::new();
    for (name, source) in &sources.entries {
        let matches = inventory
            .get(name)
            .into_iter()
            .flatten()
            .filter(|(owner, _)| profile.is_none_or(|selected| owner == selected))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            skipped.push(source.clone());
            continue;
        }
        if matches.len() > 1 && !all {
            let owners = matches
                .iter()
                .map(|(owner, _)| owner.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "update skill {name} exists in multiple profiles ({owners}); use --profile or --all"
            );
        }
        for (_, destination) in matches {
            if normalize_path(source) == normalize_path(destination) {
                skipped.push(source.clone());
                continue;
            }
            items.push(InstallItem {
                source: source.clone(),
                destination: destination.clone(),
            });
        }
    }

    if dry_run {
        for item in &items {
            println!(
                "update\t{}\t{}",
                item.source.display(),
                item.destination.display()
            );
        }
        for path in skipped {
            println!("skip\t{}", path.display());
        }
        return Ok(());
    }
    install_batch(&items, store.root(), true)
}

struct CollectedSources {
    entries: BTreeMap<String, PathBuf>,
}

fn collect_sources(
    source: &Path,
    selected_names: &[String],
    profiles_root: &Path,
) -> Result<CollectedSources> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("source does not exist: {}", source.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("source is not a directory: {}", source.display());
    }
    let mut available = BTreeMap::new();
    let mut managed_links = BTreeMap::new();
    for entry in sorted_entries(source)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        validate_component(&name, "skill")?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            let target = resolved_link_target(&entry.path())?;
            if is_managed_skill_target(&target, profiles_root) {
                managed_links.insert(name, entry.path());
                continue;
            }
            bail!(
                "source entry is an unmanaged symlink: {}",
                entry.path().display()
            );
        }
        if !metadata.is_dir() {
            bail!(
                "source entry is not a directory: {}",
                entry.path().display()
            );
        }
        available.insert(name, entry.path());
    }
    if selected_names.is_empty() {
        return Ok(CollectedSources { entries: available });
    }
    let mut selected = BTreeMap::new();
    for name in selected_names {
        validate_component(name, "skill")?;
        if let Some(path) = available.get(name) {
            selected.insert(name.clone(), path.clone());
            continue;
        }
        if managed_links.contains_key(name) {
            continue;
        }
        bail!("source skill does not exist: {name}");
    }
    Ok(CollectedSources { entries: selected })
}

fn create_profile_with_sources(
    sources: &BTreeMap<String, PathBuf>,
    destination: &Path,
) -> Result<()> {
    let root = destination
        .parent()
        .context("profile destination has no parent")?;
    let staging = root.join(format!(".sm-profile-{}.{}", std::process::id(), nonce()));
    if staging.exists() || destination.exists() {
        bail!(
            "profile transaction destination already exists: {}",
            destination.display()
        );
    }
    fs::create_dir(&staging)?;
    let result = (|| -> Result<()> {
        fs::write(staging.join(TAG_FILE), "true\n")?;
        for (name, source) in sources {
            copy_tree(source, &staging.join(name))?;
        }
        fs::rename(&staging, destination)?;
        Ok(())
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.with_context(|| format!("failed to create profile {}", destination.display()))
}

#[derive(Clone)]
struct InstallItem {
    source: PathBuf,
    destination: PathBuf,
}

fn install_batch(items: &[InstallItem], transaction_root: &Path, replace: bool) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(transaction_root)
        .with_context(|| format!("failed to create {}", transaction_root.display()))?;
    let transaction = format!("{}.{}", std::process::id(), nonce());
    let staging = transaction_root.join(format!(".sm-install-{transaction}"));
    let backup = transaction_root.join(format!(".sm-install-backup-{transaction}"));
    if staging.exists() || backup.exists() {
        bail!(
            "install transaction path already exists in {}",
            transaction_root.display()
        );
    }

    for item in items {
        match fs::symlink_metadata(&item.destination) {
            Ok(metadata) => {
                if !replace {
                    bail!("destination already exists: {}", item.destination.display());
                }
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    bail!(
                        "replace destination is not a directory: {}",
                        item.destination.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }

    fs::create_dir(&staging)?;
    let staged = (|| -> Result<()> {
        for (index, item) in items.iter().enumerate() {
            copy_tree(&item.source, &staging.join(format!("{index:08}")))?;
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
        for (index, item) in items.iter().enumerate() {
            if item.destination.exists() {
                fs::rename(&item.destination, backup.join(format!("{index:08}")))?;
                backed_up.push(index);
            }
            fs::rename(staging.join(format!("{index:08}")), &item.destination)?;
            installed.push(index);
        }
        Ok(())
    })();

    if let Err(error) = committed {
        let rollback = rollback_install(items, &backup, &installed, &backed_up);
        let _ = fs::remove_dir_all(&staging);
        if let Err(rollback_error) = rollback {
            return Err(error.context(format!(
                "install rollback failed: {rollback_error:#}; backup remains at {}",
                backup.display()
            )));
        }
        let _ = fs::remove_dir_all(&backup);
        return Err(error);
    }

    fs::remove_dir_all(&staging)?;
    fs::remove_dir_all(&backup)
        .with_context(|| format!("failed to remove install backup {}", backup.display()))
}

fn rollback_install(
    items: &[InstallItem],
    backup: &Path,
    installed: &[usize],
    backed_up: &[usize],
) -> Result<()> {
    let mut failures = Vec::new();
    for index in installed.iter().rev() {
        let path = &items[*index].destination;
        if let Err(error) = fs::remove_dir_all(path) {
            failures.push(format!("failed to remove {}: {error}", path.display()));
        }
    }
    for index in backed_up.iter().rev() {
        let destination = &items[*index].destination;
        if let Err(error) = fs::rename(backup.join(format!("{index:08}")), destination) {
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

fn preflight_export<'a>(names: impl Iterator<Item = &'a String>, destination: &Path) -> Result<()> {
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

fn preflight_import<'a>(
    names: impl Iterator<Item = &'a String>,
    destination: &Path,
    replace: bool,
) -> Result<()> {
    for name in names {
        let path = destination.join(name);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !replace {
            bail!("destination already exists: {}", path.display());
        }
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!("replace destination is not a directory: {}", path.display());
        }
    }
    Ok(())
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

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
