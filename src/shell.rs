use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::config::ShellTemplate;
use crate::paths::AppPaths;
use crate::util::{is_executable, process_is_alive, set_executable, shell_quote, symlink_dir};

pub fn run_shell(
    paths: &AppPaths,
    target_name: &str,
    template: &ShellTemplate,
    selected: &BTreeMap<String, PathBuf>,
    shell_args: &[OsString],
) -> Result<ExitStatus> {
    let executable = resolve_executable(&template.command)?;
    let current_exe = env::current_exe().context("failed to locate sm executable")?;
    let generation_id = generation_id(target_name, template, selected, &current_exe);
    let creation_lease = create_lease(paths, &generation_id, std::process::id())?;
    let generation = match ensure_generation(
        paths,
        &generation_id,
        template,
        selected,
        &executable,
        &current_exe,
    ) {
        Ok(generation) => generation,
        Err(error) => {
            let _ = fs::remove_dir(&creation_lease);
            cleanup_empty_lease_parent(&creation_lease);
            return Err(error);
        }
    };

    let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
    let original_path = env::var_os("PATH").unwrap_or_default();
    let path = prepend_path(&generation.join("bin"), &original_path)?;
    let child_result = Command::new(shell)
        .args(shell_args)
        .env("PATH", path)
        .env("SM_SKILLS_DIR", generation.join("skills"))
        .env("SM_GENERATION", &generation_id)
        .spawn()
        .context("failed to start child shell");
    let mut child = match child_result {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_dir(&creation_lease);
            cleanup_empty_lease_parent(&creation_lease);
            return Err(error);
        }
    };
    let lease = match create_lease(paths, &generation_id, child.id()) {
        Ok(lease) => lease,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_dir(&creation_lease);
            cleanup_empty_lease_parent(&creation_lease);
            return Err(error);
        }
    };
    let _ = fs::remove_dir(&creation_lease);
    let status = child.wait().context("failed to wait for child shell")?;
    let _ = fs::remove_dir(&lease);
    cleanup_empty_lease_parent(&lease);
    Ok(status)
}

pub fn exec_with_lease(paths: &AppPaths, generation: &str, command: &[OsString]) -> Result<()> {
    validate_generation_id(generation)?;
    if command.is_empty() {
        bail!("__exec requires a command");
    }
    let generation_path = paths.generations().join(generation);
    if !generation_is_complete(&generation_path) {
        bail!("generation does not exist or is incomplete: {generation}");
    }
    let lease = create_lease(paths, generation, std::process::id())?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new(&command[0]).args(&command[1..]).exec();
        let _ = fs::remove_dir(&lease);
        Err(error).context("failed to execute wrapped command")
    }
    #[cfg(not(unix))]
    {
        let status = Command::new(&command[0]).args(&command[1..]).status()?;
        let _ = fs::remove_dir(&lease);
        if status.success() {
            Ok(())
        } else {
            bail!("wrapped command exited with {status}")
        }
    }
}

pub fn gc(paths: &AppPaths, dry_run: bool) -> Result<()> {
    if !paths.generations().exists() {
        if !dry_run {
            fs::create_dir_all(paths.generations())?;
        }
        return Ok(());
    }
    if !dry_run {
        fs::create_dir_all(paths.leases())?;
    }
    for generation in crate::util::sorted_entries(&paths.generations())? {
        let id = generation.file_name().to_string_lossy().into_owned();
        if validate_generation_id(&id).is_err() || !generation_is_complete(&generation.path()) {
            continue;
        }
        let lease_root = paths.leases().join(&id);
        if has_live_leases(&lease_root, dry_run)? {
            continue;
        }
        println_if_dry_run(dry_run, &generation.path());
        if !dry_run {
            fs::remove_dir_all(generation.path()).with_context(|| {
                format!(
                    "failed to remove generation {}",
                    generation.path().display()
                )
            })?;
            if lease_root.exists() {
                let _ = fs::remove_dir(&lease_root);
            }
        }
    }
    Ok(())
}

fn ensure_generation(
    paths: &AppPaths,
    id: &str,
    template: &ShellTemplate,
    selected: &BTreeMap<String, PathBuf>,
    executable: &Path,
    current_exe: &Path,
) -> Result<PathBuf> {
    let root = paths.generations();
    fs::create_dir_all(&root)?;
    let destination = root.join(id);
    if generation_is_complete(&destination) {
        return Ok(destination);
    }
    if destination.exists() {
        bail!("incomplete generation exists: {}", destination.display());
    }

    let staging = root.join(format!(".tmp-{}-{}", id, std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir(&staging)?;
    let result = (|| -> Result<()> {
        let skills = staging.join("skills");
        let bin = staging.join("bin");
        fs::create_dir(&skills)?;
        fs::create_dir(&bin)?;
        for (name, source) in selected {
            symlink_dir(source, &skills.join(name))?;
        }
        let command_name = template
            .command
            .file_name()
            .context("shell command has no file name")?;
        let wrapper = bin.join(command_name);
        write_wrapper(
            &wrapper,
            id,
            template,
            &destination.join("skills"),
            executable,
            current_exe,
        )?;
        match fs::rename(&staging, &destination) {
            Ok(()) => Ok(()),
            Err(_) if generation_is_complete(&destination) => {
                fs::remove_dir_all(&staging)?;
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.with_context(|| format!("failed to create generation {id}"))?;
    Ok(destination)
}

fn write_wrapper(
    wrapper: &Path,
    generation: &str,
    template: &ShellTemplate,
    skills: &Path,
    executable: &Path,
    current_exe: &Path,
) -> Result<()> {
    let replace = |value: &str| value.replace("{skills}", &skills.to_string_lossy());
    let mut command = Vec::<OsString>::new();
    command.push(executable.as_os_str().to_owned());
    for argument in &template.args {
        command.push(OsString::from(replace(argument)));
    }

    let mut script = String::from("#!/bin/sh\n");
    for (key, value) in &template.env {
        script.push_str(key);
        script.push('=');
        script.push_str(&shell_quote(OsStr::new(&replace(value))));
        script.push(' ');
    }
    script.push_str("exec ");
    script.push_str(&shell_quote(current_exe.as_os_str()));
    script.push(' ');
    script.push_str("__exec ");
    script.push_str(&shell_quote(OsStr::new(generation)));
    script.push_str(" --");
    for argument in &command {
        script.push(' ');
        script.push_str(&shell_quote(argument));
    }
    script.push_str(" \"$@\"\n");
    fs::write(wrapper, script)
        .with_context(|| format!("failed to write wrapper {}", wrapper.display()))?;
    set_executable(wrapper)
}

fn generation_id(
    target_name: &str,
    template: &ShellTemplate,
    selected: &BTreeMap<String, PathBuf>,
    current_exe: &Path,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(target_name.as_bytes());
    hasher.update(current_exe.as_os_str().as_encoded_bytes());
    hasher.update(template.command.as_os_str().as_encoded_bytes());
    for argument in &template.args {
        hasher.update(argument.as_bytes());
        hasher.update(&[0]);
    }
    for (key, value) in &template.env {
        hasher.update(key.as_bytes());
        hasher.update(&[0]);
        hasher.update(value.as_bytes());
        hasher.update(&[0]);
    }
    for (name, source) in selected {
        hasher.update(name.as_bytes());
        hasher.update(&[0]);
        hasher.update(source.as_os_str().as_encoded_bytes());
        hasher.update(&[0]);
    }
    hasher.finalize().to_hex()[..24].to_owned()
}

fn resolve_executable(command: &Path) -> Result<PathBuf> {
    if command.components().count() > 1 || command.is_absolute() {
        let path = crate::util::absolute_path(command)?;
        if is_executable(&path) {
            return Ok(path);
        }
        bail!("configured command is not executable: {}", path.display());
    }
    let path = env::var_os("PATH").context("PATH is not set")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(command);
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }
    bail!(
        "configured command cannot be resolved: {}",
        command.display()
    )
}

fn prepend_path(directory: &Path, original: &OsStr) -> Result<OsString> {
    let mut paths = vec![directory.to_path_buf()];
    paths.extend(env::split_paths(original));
    env::join_paths(paths).context("failed to construct child PATH")
}

fn create_lease(paths: &AppPaths, generation: &str, pid: u32) -> Result<PathBuf> {
    validate_generation_id(generation)?;
    let root = paths.leases().join(generation);
    fs::create_dir_all(&root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let lease = root.join(format!("{pid}-{nonce}"));
    fs::create_dir(&lease)?;
    Ok(lease)
}

fn has_live_leases(root: &Path, dry_run: bool) -> Result<bool> {
    if !root.exists() {
        return Ok(false);
    }
    let mut live = false;
    for entry in crate::util::sorted_entries(root)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let pid = name
            .split('-')
            .next()
            .and_then(|value| value.parse::<u32>().ok());
        if pid.is_some_and(process_is_alive) {
            live = true;
            continue;
        }
        if !dry_run {
            if entry.path().is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(live)
}

fn cleanup_empty_lease_parent(lease: &Path) {
    if let Some(parent) = lease.parent() {
        let _ = fs::remove_dir(parent);
    }
}

fn validate_generation_id(generation: &str) -> Result<()> {
    if generation.len() != 24
        || !generation
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("invalid generation ID: {generation}");
    }
    Ok(())
}

fn generation_is_complete(path: &Path) -> bool {
    is_real_directory(path)
        && is_real_directory(&path.join("skills"))
        && is_real_directory(&path.join("bin"))
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

fn println_if_dry_run(dry_run: bool, path: &Path) {
    if dry_run {
        println!("remove\t{}", path.display());
    }
}
