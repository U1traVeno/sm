mod cli;
mod config;
mod materialize;
mod paths;
mod profiles;
mod shell;
mod state;
mod transfer;
mod util;

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command};
use config::Config;
use materialize::{apply_links, managed_status, plan_links};
use paths::AppPaths;
use profiles::ProfileStore;
use state::{
    ProfileLock, TargetLock, disable_profiles, enable_profiles, load_enabled, save_enabled,
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("sm: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let store = ProfileStore::new(paths.profiles());

    match cli.command {
        Command::Profiles => {
            for profile in store.list_profiles()? {
                println!("{profile}");
            }
        }
        Command::Skills { profiles } => {
            let profiles = if profiles.is_empty() {
                store.list_profiles()?
            } else {
                profiles
            };
            for profile in profiles {
                for skill in store.skills(&profile)?.into_keys() {
                    println!("{profile}/{skill}");
                }
            }
        }
        Command::Targets => {
            let config = Config::load(&paths.config)?;
            for target in config.targets.keys() {
                println!("{target}");
            }
        }
        Command::Enabled { target } => {
            let config = Config::load(&paths.config)?;
            let (name, target) = config.select_target(target.as_deref())?;
            target.persistent_path(name)?;
            for entry in load_enabled(&paths.target_state(name))? {
                println!("{}", entry.name);
            }
        }
        Command::Status { target } => {
            let config = Config::load(&paths.config)?;
            let (name, target) = config.select_target(target.as_deref())?;
            let target_path = target.persistent_path(name)?;
            for (skill, source, destination) in managed_status(&target_path, store.root())? {
                println!("{skill}\t{}\t{}", source.display(), destination.display());
            }
        }
        Command::Enable {
            profiles,
            target,
            dry_run,
        } => {
            let config = Config::load(&paths.config)?;
            let (name, target) = config.select_target(target.as_deref())?;
            mutate_enabled(
                &paths,
                &store,
                name,
                &target.persistent_path(name)?,
                &profiles,
                true,
                dry_run,
            )?;
        }
        Command::Disable {
            profiles,
            target,
            dry_run,
        } => {
            let config = Config::load(&paths.config)?;
            let (name, target) = config.select_target(target.as_deref())?;
            mutate_enabled(
                &paths,
                &store,
                name,
                &target.persistent_path(name)?,
                &profiles,
                false,
                dry_run,
            )?;
        }
        Command::Apply { target, dry_run } => {
            let config = Config::load(&paths.config)?;
            let (name, target) = config.select_target(target.as_deref())?;
            reconcile_existing(
                &paths,
                &store,
                name,
                &target.persistent_path(name)?,
                dry_run,
            )?;
        }
        Command::Shell {
            target,
            profiles: _,
            skills: _,
            shell_args,
        } => {
            let config = Config::load(&paths.config)?;
            let template = config
                .target(&target)?
                .shell
                .as_ref()
                .with_context(|| format!("target {target} has no shell template"))?;
            let inherited = load_enabled(&paths.target_state(&target))?
                .into_iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>();
            let selectors = cli::selectors_from_process_args("shell");
            let selected = store.resolve_selectors(&inherited, &selectors)?;
            let status = shell::run_shell(&paths, &target, template, &selected, &shell_args)?;
            return Ok(exit_status_code(status));
        }
        Command::Export {
            profiles: _,
            skills: _,
            destination,
            dry_run,
        } => {
            let selectors = cli::selectors_from_process_args("export");
            if selectors.is_empty() {
                anyhow::bail!("export requires at least one --profile or --skill selector");
            }
            let selected = store.resolve_selectors(&[], &selectors)?;
            transfer::export_skills(&selected, Path::new(&destination), dry_run)?;
        }
        Command::Import {
            profile,
            skills,
            source,
            replace,
            dry_run,
        } => {
            let profile_state = paths.profile_state(&profile)?;
            let _lock = if dry_run {
                None
            } else {
                Some(ProfileLock::acquire(&profile_state)?)
            };
            transfer::import_skills(
                &store,
                &profile,
                Path::new(&source),
                &skills,
                replace,
                dry_run,
            )?;
        }
        Command::Gc { dry_run } => shell::gc(&paths, dry_run)?,
        Command::Exec {
            generation,
            command,
        } => {
            shell::exec_with_lease(&paths, &generation, &command)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[allow(clippy::too_many_arguments)]
fn mutate_enabled(
    paths: &AppPaths,
    store: &ProfileStore,
    target_name: &str,
    target_path: &Path,
    profiles: &[String],
    enable: bool,
    dry_run: bool,
) -> Result<()> {
    let target_state = paths.target_state(target_name);
    let _lock = if dry_run {
        None
    } else {
        Some(TargetLock::acquire(&target_state)?)
    };
    let mut entries = load_enabled(&target_state)?;
    if enable {
        for profile in profiles {
            store.skills(profile)?;
        }
        enable_profiles(&mut entries, profiles)?;
    } else {
        disable_profiles(&mut entries, profiles)?;
    }
    let profile_order = entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    let desired = store.resolve_profiles(&profile_order)?;
    let operations = plan_links(&desired, target_path, store.root())?;

    if dry_run {
        for profile in profiles {
            println!(
                "{}\t{}\t{}",
                if enable { "enable" } else { "disable" },
                target_name,
                profile
            );
        }
        apply_links(&operations, target_path, true)?;
        return Ok(());
    }

    save_enabled(&target_state, &entries)?;
    apply_links(&operations, target_path, false)
}

fn reconcile_existing(
    paths: &AppPaths,
    store: &ProfileStore,
    target_name: &str,
    target_path: &Path,
    dry_run: bool,
) -> Result<()> {
    let target_state = paths.target_state(target_name);
    let _lock = if dry_run {
        None
    } else {
        Some(TargetLock::acquire(&target_state)?)
    };
    let entries = load_enabled(&target_state)?;
    let profiles = entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    let desired = store.resolve_profiles(&profiles)?;
    let operations = plan_links(&desired, target_path, store.root())?;
    apply_links(&operations, target_path, dry_run)
}

fn exit_status_code(status: std::process::ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        _ if status.success() => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    }
}
