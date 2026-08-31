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

use anyhow::{Result, bail};
use clap::Parser;

use cli::{Cli, Command, ProfilesCommand};
use config::Config;
use materialize::{apply_links, managed_status, plan_links};
use paths::AppPaths;
use profiles::ProfileStore;
use state::{InventoryLock, TargetLock, remove_legacy_activation_state};

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
        Command::Profiles { command } => match command {
            None => {
                let _lock = initialize_inventory(&paths, &store, false)?;
                for profile in store.list_profiles()? {
                    println!("{profile}");
                }
            }
            Some(ProfilesCommand::New {
                profiles,
                disabled,
                dry_run,
            }) => {
                let _lock = initialize_inventory(&paths, &store, dry_run)?;
                store.create_profiles(&profiles, !disabled, dry_run)?;
            }
        },
        Command::Skills { profiles } => {
            let _lock = initialize_inventory(&paths, &store, false)?;
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
        Command::Enabled => {
            let _lock = initialize_inventory(&paths, &store, false)?;
            for profile in store.enabled_profiles()? {
                println!("{profile}");
            }
        }
        Command::Status { target } => {
            let _lock = initialize_inventory(&paths, &store, false)?;
            let config = Config::load(&paths.config)?;
            let targets = config.persistent_targets(target.as_deref())?;
            let show_target = target.is_none();
            for (target_name, target_path) in targets {
                for (skill, source, destination) in managed_status(&target_path, store.root())? {
                    if show_target {
                        println!(
                            "{target_name}\t{skill}\t{}\t{}",
                            source.display(),
                            destination.display()
                        );
                    } else {
                        println!("{skill}\t{}\t{}", source.display(), destination.display());
                    }
                }
            }
        }
        Command::Enable { profiles, dry_run } => {
            let _lock = initialize_inventory(&paths, &store, dry_run)?;
            store.set_enabled(&profiles, true, dry_run)?;
        }
        Command::Disable { profiles, dry_run } => {
            let _lock = initialize_inventory(&paths, &store, dry_run)?;
            store.set_enabled(&profiles, false, dry_run)?;
        }
        Command::Apply {
            target,
            force,
            dry_run,
        } => {
            let _inventory_lock = initialize_inventory(&paths, &store, dry_run)?;
            let desired = store.resolve_enabled()?;
            let config = Config::load(&paths.config)?;
            let targets = config.persistent_targets(target.as_deref())?;
            let _target_locks = if dry_run {
                Vec::new()
            } else {
                targets
                    .iter()
                    .map(|(name, _)| TargetLock::acquire(&paths.target_state(name)))
                    .collect::<Result<Vec<_>>>()?
            };
            let plans = targets
                .iter()
                .map(|(_, path)| {
                    plan_links(&desired, path, store.root(), force)
                        .map(|operations| (path.clone(), operations))
                })
                .collect::<Result<Vec<_>>>()?;
            for (target, operations) in plans {
                apply_links(&operations, &target, dry_run)?;
            }
        }
        Command::Shell {
            target,
            mut targets,
            profiles: _,
            skills: _,
            shell_args,
        } => {
            if let Some(target) = target {
                targets.push(target);
            }
            let inventory_lock = initialize_inventory(&paths, &store, false)?;
            let config = Config::load(&paths.config)?;
            let adapters = config.shell_adapters(&targets)?;
            let inherited = store.enabled_profiles()?;
            let selectors = cli::selectors_from_process_args("shell");
            let selected = store.resolve_selectors(&inherited, &selectors)?;
            drop(inventory_lock);
            let status = shell::run_shell(&paths, &adapters, &selected, &shell_args)?;
            return Ok(exit_status_code(status));
        }
        Command::Export {
            profiles: _,
            skills: _,
            destination,
            dry_run,
        } => {
            let _lock = initialize_inventory(&paths, &store, dry_run)?;
            let selectors = cli::selectors_from_process_args("export");
            if selectors.is_empty() {
                bail!("export requires at least one --profile or --skill selector");
            }
            let selected = store.resolve_selectors(&[], &selectors)?;
            transfer::export_skills(&selected, Path::new(&destination), dry_run)?;
        }
        Command::Import {
            profile,
            skills,
            source,
            create,
            replace,
            dry_run,
        } => {
            let _lock = initialize_inventory(&paths, &store, dry_run)?;
            transfer::import_skills(
                &store,
                &profile,
                Path::new(&source),
                &skills,
                create,
                replace,
                dry_run,
            )?;
        }
        Command::Update {
            source,
            skills,
            profile,
            all,
            dry_run,
        } => {
            let _lock = initialize_inventory(&paths, &store, dry_run)?;
            transfer::update_skills(
                &store,
                Path::new(&source),
                &skills,
                profile.as_deref(),
                all,
                dry_run,
            )?;
        }
        Command::Adopt {
            target,
            directory,
            dry_run,
        } => {
            Config::adopt(&paths.config, &target, Path::new(&directory), dry_run)?;
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

fn initialize_inventory(
    paths: &AppPaths,
    store: &ProfileStore,
    dry_run: bool,
) -> Result<Option<InventoryLock>> {
    let lock = if dry_run {
        None
    } else {
        Some(InventoryLock::acquire(&paths.inventory_state())?)
    };
    store.initialize_missing_tags(dry_run)?;
    remove_legacy_activation_state(&paths.state, dry_run)?;
    Ok(lock)
}

fn exit_status_code(status: std::process::ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        _ if status.success() => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    }
}
