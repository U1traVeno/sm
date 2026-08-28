use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::cli::Selector;
use crate::util::{sorted_entries, validate_component};

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

    pub fn skills(&self, profile: &str) -> Result<BTreeMap<String, PathBuf>> {
        validate_component(profile, "profile")?;
        let path = self.root.join(profile);
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("profile does not exist: {profile}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("profile is not a directory: {}", path.display());
        }

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
}
