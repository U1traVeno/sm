use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

pub fn user_home() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .context("HOME is not set to an absolute path")
}

pub fn expand_home(path: &Path) -> Result<PathBuf> {
    let text = path.as_os_str().to_string_lossy();
    if text == "~" {
        return user_home();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return Ok(user_home()?.join(rest));
    }
    Ok(path.to_path_buf())
}

pub fn absolute_path(path: &Path) -> Result<PathBuf> {
    let expanded = expand_home(path)?;
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        env::current_dir()
            .context("failed to read current directory")?
            .join(expanded)
    };
    Ok(normalize_path(&absolute))
}

pub fn require_absolute_config_path(path: &Path, field: &str) -> Result<PathBuf> {
    let expanded = expand_home(path)?;
    if !expanded.is_absolute() {
        bail!("{field} must be an absolute path: {}", path.display());
    }
    Ok(normalize_path(&expanded))
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

pub fn validate_component(value: &str, kind: &str) -> Result<()> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.starts_with('.')
        || value.contains('/')
        || value.contains('\\')
        || Path::new(value).components().count() != 1
    {
        bail!("invalid {kind} name: {value}");
    }
    Ok(())
}

pub fn sorted_entries(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("failed to read directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to read directory {}", path.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

pub fn shell_quote(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
pub fn symlink_dir(source: &Path, destination: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, destination).with_context(|| {
        format!(
            "failed to link {} -> {}",
            destination.display(),
            source.display()
        )
    })
}

#[cfg(windows)]
pub fn symlink_dir(source: &Path, destination: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(source, destination).with_context(|| {
        format!(
            "failed to link {} -> {}",
            destination.display(),
            source.display()
        )
    })
}

#[cfg(unix)]
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
pub fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to make {} executable", path.display()))
}

#[cfg(windows)]
pub fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn process_is_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
pub fn process_is_alive(_pid: u32) -> bool {
    false
}
