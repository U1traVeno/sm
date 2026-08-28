use std::ffi::OsString;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "sm",
    version,
    about = "Activate skill profiles on demand",
    long_about = "Activate skill profiles on demand.\n\nsm keeps the complete skill inventory under $SM_HOME/profiles and exposes only the union of enabled profiles to each configured target. Persistent activation uses managed symlinks; isolated shells use stable generated symlink sets.\n\nsm does not install skill packages, parse SKILL.md, or run Git. Successful mutating commands are silent unless --dry-run is used.",
    after_long_help = "Common workflows:\n  sm profiles\n  sm enable common coding --target pi\n  sm shell pi --profile common --profile research\n  sm export --profile common --skill research/web-search\n\nLocations:\n  Skills:  ${SM_HOME:-~/.sm}/profiles/\n  Config:  ${XDG_CONFIG_HOME:-~/.config}/sm/config.toml\n  State:   ${XDG_STATE_HOME:-~/.local/state}/sm/\n  Cache:   ${XDG_CACHE_HOME:-~/.cache}/sm/\n\nRun 'sm <command> --help' for command-specific behavior and examples.",
    arg_required_else_help = true,
    subcommand_required = true,
    propagate_version = true,
    next_line_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List available profiles.
    #[command(
        long_about = "List available profiles.\n\nA profile is an immediate, non-hidden directory under $SM_HOME/profiles. Names are written to standard output in bytewise lexical order, one per line. A missing or empty profiles directory produces no output and succeeds.",
        after_long_help = "Examples:\n  sm profiles\n  sm profiles | grep '^work-'"
    )]
    Profiles,

    /// List profile-qualified skills.
    #[command(
        long_about = "List profile-qualified skills.\n\nEach output line has the form <profile>/<skill>. With no PROFILE arguments, skills from every profile are listed. sm identifies skills by immediate directory name and does not inspect SKILL.md.",
        after_long_help = "Examples:\n  sm skills\n  sm skills coding research\n  sm skills common | sort"
    )]
    Skills {
        /// Profiles to inspect; omit to inspect every profile.
        #[arg(value_name = "PROFILE")]
        profiles: Vec<String>,
    },

    /// List configured targets.
    #[command(
        long_about = "List configured targets.\n\nTarget names come from ${XDG_CONFIG_HOME:-~/.config}/sm/config.toml and are written one per line in lexical order. A malformed target configuration is reported on standard error.",
        after_long_help = "Examples:\n  sm targets\n  sm targets | grep '^pi$'"
    )]
    Targets,

    /// List enabled profiles from lowest to highest precedence.
    #[command(
        long_about = "List enabled profiles for one persistent target.\n\nProfiles are printed from lowest to highest precedence, one per line. The order is stored as empty marker directories under the XDG state directory. Later-enabled profiles win duplicate skill names.",
        after_long_help = "Examples:\n  sm enabled\n  sm enabled --target pi\n  sm enabled -t claude | tail -n 1"
    )]
    Enabled {
        /// Persistent target name; otherwise use normal target selection.
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
    },

    /// List materialized managed links.
    #[command(
        long_about = "List materialized managed links for one persistent target.\n\nEach line is tab-separated as <skill-name> <absolute-source> <absolute-destination>. Only symlinks managed by sm are shown; unmanaged files, directories, and links are omitted. An empty target produces no output.",
        after_long_help = "Examples:\n  sm status\n  sm status --target pi\n  sm status -t pi | cut -f1"
    )]
    Status {
        /// Persistent target name; otherwise use normal target selection.
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
    },

    /// Enable profiles for a persistent target.
    #[command(
        long_about = "Enable profiles for one persistent target.\n\nPROFILE arguments are processed from left to right. Each profile moves to the highest precedence, so the last argument wins duplicate skill names. sm validates the complete result and unmanaged collisions before saving marker state and reconciling managed symlinks.",
        after_long_help = "Examples:\n  sm enable common coding\n  sm enable common coding --target pi\n  sm enable research -t claude --dry-run"
    )]
    Enable {
        /// Profiles to enable, in increasing precedence order.
        #[arg(required = true, value_name = "PROFILE")]
        profiles: Vec<String>,
        /// Persistent target name; otherwise use normal target selection.
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
        /// Validate and print planned operations without changing files.
        #[arg(long)]
        dry_run: bool,
    },

    /// Disable profiles for a persistent target.
    #[command(
        long_about = "Disable profiles for one persistent target.\n\nRemoving a winning profile reveals the next highest-precedence copy of each duplicate skill. Disabling an already-disabled profile is an idempotent success and still reconciles the target. Unmanaged target entries are never removed.",
        after_long_help = "Examples:\n  sm disable research\n  sm disable work personal --target pi\n  sm disable coding -t claude --dry-run"
    )]
    Disable {
        /// Profiles to disable.
        #[arg(required = true, value_name = "PROFILE")]
        profiles: Vec<String>,
        /// Persistent target name; otherwise use normal target selection.
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
        /// Validate and print planned operations without changing files.
        #[arg(long)]
        dry_run: bool,
    },

    /// Reconcile a persistent target without changing precedence.
    #[command(
        long_about = "Reconcile a persistent target without changing enabled profiles or precedence.\n\nUse apply after editing or updating profile directories, or to repair an interrupted target update. sm recreates missing or stale managed links, removes obsolete managed links, and preserves unmanaged entries.",
        after_long_help = "Examples:\n  git -C ~/.sm pull --ff-only && sm apply\n  sm apply --target pi\n  sm apply -t claude --dry-run"
    )]
    Apply {
        /// Persistent target name; otherwise use normal target selection.
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
        /// Validate and print planned operations without changing files.
        #[arg(long)]
        dry_run: bool,
    },

    /// Start an isolated child shell.
    #[command(
        long_about = "Start an isolated child shell using a target's generic command template.\n\nsm creates or reuses a stable generated skill directory and prepends a private wrapper to the child shell's PATH. Only the configured command is wrapped.\n\nWith any --profile option, persistent enabled profiles are not inherited. Without --profile, the target's persistent enabled profiles form the base set. --skill selections are added in either mode. Later selectors win duplicate directory names. The target must accept an explicit skill path through its argv or environment template.",
        after_long_help = "Examples:\n  sm shell pi\n  sm shell pi --skill research/web-search\n  sm shell pi --profile common --profile coding\n  sm shell pi --profile common -- -l\n\nInside the child shell, invoke the configured command normally. Exit the child shell to end its scope."
    )]
    Shell {
        /// Target whose shell command template will be wrapped.
        #[arg(value_name = "TARGET")]
        target: String,
        /// Explicit profile selection; repeatable. Any occurrence disables inheritance.
        #[arg(long = "profile", value_name = "PROFILE")]
        profiles: Vec<String>,
        /// Add one profile-qualified skill; repeatable.
        #[arg(long = "skill", value_name = "PROFILE/SKILL")]
        skills: Vec<String>,
        /// Arguments after -- are passed to the child shell.
        #[arg(last = true, value_name = "SHELL_ARGUMENT")]
        shell_args: Vec<OsString>,
    },

    /// Copy profiles or individual skills into a project directory.
    #[command(
        long_about = "Copy selected profiles or individual skills into a project-owned directory.\n\nAt least one --profile or --skill selector is required. Selectors form a left-to-right union; later selectors win duplicate skill names. The destination defaults to .skills/. Every collision is checked before copying. Existing entries are never merged or overwritten.\n\nExport is a one-time ownership transfer: sm does not track, update, or later delete the copied directories.",
        after_long_help = "Examples:\n  sm export --profile common\n  sm export --skill research/web-search\n  sm export --profile coding --to .agents/skills\n  sm export --profile common --dry-run"
    )]
    Export {
        /// Copy every skill in this profile; repeatable.
        #[arg(long = "profile", value_name = "PROFILE")]
        profiles: Vec<String>,
        /// Copy one profile-qualified skill; repeatable.
        #[arg(long = "skill", value_name = "PROFILE/SKILL")]
        skills: Vec<String>,
        /// Project-owned destination directory.
        #[arg(long = "to", default_value = ".skills", value_name = "DIRECTORY")]
        destination: OsString,
        /// Validate and print planned copies without changing files.
        #[arg(long)]
        dry_run: bool,
    },

    /// Copy project skills into a profile.
    #[command(
        long_about = "Copy project-owned skills into one profile.\n\nThe source defaults to .skills/. With no --skill options, every immediate skill directory is imported. With --skill, only the named source entries are copied. The destination profile is created when absent. Any existing destination name fails the entire operation.\n\nImport is a one-time ownership transfer: the destination profile owns the copies and sm does not synchronize them with the project.",
        after_long_help = "Examples:\n  sm import --profile project-tools\n  sm import --profile project-tools --skill deploy --skill release-notes\n  sm import --profile coding --from .agents/skills\n  sm import --profile coding --dry-run"
    )]
    Import {
        /// Destination profile, created when absent.
        #[arg(long, value_name = "PROFILE")]
        profile: String,
        /// Import one unqualified skill name; repeatable. Omit to import all.
        #[arg(long = "skill", value_name = "SKILL")]
        skills: Vec<String>,
        /// Project-owned source directory.
        #[arg(long = "from", default_value = ".skills", value_name = "DIRECTORY")]
        source: OsString,
        /// Validate and print planned copies without changing files.
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove shell generations without live leases.
    #[command(
        long_about = "Remove stable shell generations that have no live process lease.\n\nRunning child shells and wrapped commands hold PID-based directory leases and are skipped. Unknown or malformed cache entries are preserved. gc removes complete generation directories only and is silent when nothing is eligible.",
        after_long_help = "Examples:\n  sm gc --dry-run\n  sm gc"
    )]
    Gc {
        /// Print removable generation directories without deleting them.
        #[arg(long)]
        dry_run: bool,
    },

    #[command(name = "__exec", hide = true)]
    Exec {
        generation: String,
        #[arg(last = true, required = true)]
        command: Vec<OsString>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    Profile(String),
    Skill(String),
}

pub fn selectors_from_process_args(command_name: &str) -> Vec<Selector> {
    let args: Vec<OsString> = std::env::args_os().collect();
    let Some(command_index) = args
        .iter()
        .position(|value| value.to_string_lossy() == command_name)
    else {
        return Vec::new();
    };

    let mut selectors = Vec::new();
    let mut index = command_index + 1;
    while index < args.len() {
        let text = args[index].to_string_lossy();
        if text == "--" {
            break;
        }
        if text == "--profile" || text == "--skill" {
            if let Some(value) = args.get(index + 1) {
                let value = value.to_string_lossy().into_owned();
                if text == "--profile" {
                    selectors.push(Selector::Profile(value));
                } else {
                    selectors.push(Selector::Skill(value));
                }
                index += 2;
                continue;
            }
        } else if let Some(value) = text.strip_prefix("--profile=") {
            selectors.push(Selector::Profile(value.to_owned()));
        } else if let Some(value) = text.strip_prefix("--skill=") {
            selectors.push(Selector::Skill(value.to_owned()));
        }
        index += 1;
    }
    selectors
}
