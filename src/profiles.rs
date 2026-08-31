use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::cli::Selector;
use crate::util::{sorted_entries, validate_component};

pub const TAG_FILE: &str = ".smtag";

#[derive(Clone, Debug)]
pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn initialize_missing_tags(&self, dry_run: bool) -> Result<()> {
        for profile in self.list_profiles()? {
            let tag = self.root.join(&profile).join(TAG_FILE);
            if tag.exists() {
                read_tag(&tag)?;
                continue;
            }
            if dry_run {
                println!("tag\ttrue\t{}", tag.display());
            } else {
                fs::write(&tag, "true\n")
                    .with_context(|| format!("failed to initialize {}", tag.display()))?;
                eprintln!("sm: initialized profile {profile} as enabled");
            }
        }
        Ok(())
    }

    pub fn list_profiles(&self) -> Result<Vec<String>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut profiles = Vec::new();
        for entry in sorted_entries(&self.root)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!(
                    "profile entry is not a directory: {}",
                    entry.path().display()
                );
            }
            validate_component(&name, "profile")?;
            profiles.push(name);
        }
        Ok(profiles)
    }

    pub fn enabled_profiles(&self) -> Result<Vec<String>> {
        let mut enabled = Vec::new();
        for profile in self.list_profiles()? {
            if self.profile_enabled(&profile)? {
                enabled.push(profile);
            }
        }
        Ok(enabled)
    }

    pub fn profile_enabled(&self, profile: &str) -> Result<bool> {
        validate_component(profile, "profile")?;
        let path = self.profile_path(profile)?;
        let tag = path.join(TAG_FILE);
        if !tag.exists() {
            return Ok(true);
        }
        read_tag(&tag)
    }

    pub fn profile_exists(&self, profile: &str) -> Result<bool> {
        validate_component(profile, "profile")?;
        let path = self.root.join(profile);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(true),
            Ok(_) => bail!("profile is not a directory: {}", path.display()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => {
                Err(error).with_context(|| format!("failed to inspect {}", path.display()))
            }
        }
    }

    pub fn create_profiles(&self, profiles: &[String], enabled: bool, dry_run: bool) -> Result<()> {
        let mut unique = BTreeSet::new();
        for profile in profiles {
            validate_component(profile, "profile")?;
            if !unique.insert(profile) {
                bail!("duplicate profile argument: {profile}");
            }
            if self.profile_exists(profile)? {
                bail!("profile already exists: {profile}");
            }
        }
        if dry_run {
            for profile in profiles {
                println!(
                    "create-profile\t{}\t{}",
                    if enabled { "true" } else { "false" },
                    self.root.join(profile).display()
                );
            }
            return Ok(());
        }

        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create {}", self.root.display()))?;
        let mut created = Vec::new();
        for profile in profiles {
            let path = self.root.join(profile);
            let result = (|| -> Result<()> {
                fs::create_dir(&path)?;
                fs::write(
                    path.join(TAG_FILE),
                    if enabled { "true\n" } else { "false\n" },
                )?;
                Ok(())
            })();
            if let Err(error) = result {
                let _ = fs::remove_dir_all(&path);
                for prior in created.iter().rev() {
                    let _ = fs::remove_dir_all(prior);
                }
                return Err(error).with_context(|| format!("failed to create profile {profile}"));
            }
            created.push(path);
        }
        Ok(())
    }

    pub fn set_enabled(&self, profiles: &[String], enabled: bool, dry_run: bool) -> Result<()> {
        let mut unique = BTreeSet::new();
        let mut overrides = BTreeMap::new();
        for profile in profiles {
            if !unique.insert(profile) {
                bail!("duplicate profile argument: {profile}");
            }
            self.profile_path(profile)?;
            overrides.insert(profile.clone(), enabled);
        }
        self.validate_enabled_overrides(&overrides)?;

        if dry_run {
            for profile in profiles {
                println!(
                    "{}\t{}",
                    if enabled { "enable" } else { "disable" },
                    profile
                );
            }
            return Ok(());
        }

        let transaction = format!("{}.{}", std::process::id(), nonce());
        let mut changes = Vec::new();
        for profile in profiles {
            let directory = self.root.join(profile);
            let tag = directory.join(TAG_FILE);
            let temporary = directory.join(format!(".smtag.tmp.{transaction}"));
            let backup = directory.join(format!(".smtag.old.{transaction}"));
            if let Err(error) = fs::write(&temporary, if enabled { "true\n" } else { "false\n" }) {
                for (_, staged, _) in &changes {
                    let _ = fs::remove_file(staged);
                }
                return Err(error).with_context(|| format!("failed to stage {}", tag.display()));
            }
            changes.push((tag, temporary, backup));
        }

        let mut installed = Vec::new();
        let mut backed_up = Vec::new();
        let committed = (|| -> Result<()> {
            for (index, (tag, temporary, backup)) in changes.iter().enumerate() {
                if fs::symlink_metadata(tag).is_ok() {
                    fs::rename(tag, backup)?;
                    backed_up.push(index);
                }
                fs::rename(temporary, tag)?;
                installed.push(index);
            }
            Ok(())
        })();
        if let Err(error) = committed {
            for index in installed.iter().rev() {
                let _ = fs::remove_file(&changes[*index].0);
            }
            for index in backed_up.iter().rev() {
                let _ = fs::rename(&changes[*index].2, &changes[*index].0);
            }
            for (_, temporary, _) in &changes {
                let _ = fs::remove_file(temporary);
            }
            return Err(error).context("failed to update profile activation atomically");
        }
        for (_, _, backup) in &changes {
            if backup.exists() {
                fs::remove_file(backup)?;
            }
        }
        Ok(())
    }

    pub fn skills(&self, profile: &str) -> Result<BTreeMap<String, PathBuf>> {
        let path = self.profile_path(profile)?;
        let mut skills = BTreeMap::new();
        for entry in sorted_entries(&path)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            validate_component(&name, "skill")?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("skill entry is not a directory: {}", entry.path().display());
            }
            skills.insert(name, entry.path());
        }
        Ok(skills)
    }

    pub fn skill(&self, reference: &str) -> Result<(String, PathBuf)> {
        let mut parts = reference.split('/');
        let profile = parts.next().unwrap_or_default();
        let skill = parts.next().unwrap_or_default();
        if parts.next().is_some() || profile.is_empty() || skill.is_empty() {
            bail!("invalid skill reference, expected <profile>/<skill>: {reference}");
        }
        validate_component(profile, "profile")?;
        validate_component(skill, "skill")?;
        let skills = self.skills(profile)?;
        let path = skills
            .get(skill)
            .with_context(|| format!("skill does not exist: {reference}"))?;
        Ok((skill.to_owned(), path.clone()))
    }

    pub fn resolve_enabled(&self) -> Result<BTreeMap<String, PathBuf>> {
        let profiles = self.enabled_profiles()?;
        self.resolve_profiles_without_duplicates(&profiles)
    }

    pub fn resolve_profiles(&self, profiles: &[String]) -> Result<BTreeMap<String, PathBuf>> {
        let mut resolved = BTreeMap::new();
        for profile in profiles {
            for (name, path) in self.skills(profile)? {
                resolved.insert(name, path);
            }
        }
        Ok(resolved)
    }

    pub fn resolve_selectors(
        &self,
        inherited: &[String],
        selectors: &[Selector],
    ) -> Result<BTreeMap<String, PathBuf>> {
        let has_explicit_profile = selectors
            .iter()
            .any(|selector| matches!(selector, Selector::Profile(_)));
        let mut resolved = if has_explicit_profile {
            BTreeMap::new()
        } else {
            self.resolve_profiles(inherited)?
        };
        for selector in selectors {
            match selector {
                Selector::Profile(profile) => {
                    for (name, path) in self.skills(profile)? {
                        resolved.insert(name, path);
                    }
                }
                Selector::Skill(reference) => {
                    let (name, path) = self.skill(reference)?;
                    resolved.insert(name, path);
                }
            }
        }
        Ok(resolved)
    }

    pub fn validate_enabled_additions<'a>(
        &self,
        profile: &str,
        profile_enabled: bool,
        additions: impl Iterator<Item = &'a String>,
    ) -> Result<()> {
        if !profile_enabled {
            return self.resolve_enabled().map(|_| ());
        }
        let additions = additions.cloned().collect::<BTreeSet<_>>();
        let mut seen = BTreeMap::<String, String>::new();
        let mut profiles = self.list_profiles()?;
        if !profiles.iter().any(|name| name == profile) {
            profiles.push(profile.to_owned());
            profiles.sort();
        }
        for current in profiles {
            let enabled = if current == profile {
                profile_enabled
            } else {
                self.profile_enabled(&current)?
            };
            if !enabled {
                continue;
            }
            let mut names = if self.profile_exists(&current)? {
                self.skills(&current)?.into_keys().collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            };
            if current == profile {
                names.extend(additions.iter().cloned());
            }
            for name in names {
                if let Some(previous) = seen.insert(name.clone(), current.clone()) {
                    bail!(
                        "enabled profiles contain duplicate skill {name}: {previous}/{name} and {current}/{name}"
                    );
                }
            }
        }
        Ok(())
    }

    fn profile_path(&self, profile: &str) -> Result<PathBuf> {
        validate_component(profile, "profile")?;
        let path = self.root.join(profile);
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("profile does not exist: {profile}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("profile is not a directory: {}", path.display());
        }
        Ok(path)
    }

    fn validate_enabled_overrides(&self, overrides: &BTreeMap<String, bool>) -> Result<()> {
        let profiles = self
            .list_profiles()?
            .into_iter()
            .filter_map(|profile| {
                let enabled = overrides
                    .get(&profile)
                    .copied()
                    .map(Ok)
                    .unwrap_or_else(|| self.profile_enabled(&profile));
                match enabled {
                    Ok(true) => Some(Ok(profile)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        self.resolve_profiles_without_duplicates(&profiles)
            .map(|_| ())
    }

    fn resolve_profiles_without_duplicates(
        &self,
        profiles: &[String],
    ) -> Result<BTreeMap<String, PathBuf>> {
        let mut resolved = BTreeMap::new();
        let mut owners = BTreeMap::<String, String>::new();
        for profile in profiles {
            for (name, path) in self.skills(profile)? {
                if let Some(previous) = owners.insert(name.clone(), profile.clone()) {
                    bail!(
                        "enabled profiles contain duplicate skill {name}: {previous}/{name} and {profile}/{name}"
                    );
                }
                resolved.insert(name, path);
            }
        }
        Ok(resolved)
    }
}

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn read_tag(path: &Path) -> Result<bool> {
    let value =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        value => bail!(
            "invalid profile tag in {}: expected true or false, found {value:?}",
            path.display()
        ),
    }
}
