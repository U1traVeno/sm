use std::ffi::OsString;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "sm",
    version,
    about = "Manage global skill profiles and project them to agent directories",
    long_about = "Manage global skill profiles and project them to agent directories.\n\nProfiles live under $SM_HOME/profiles and store global activation in .smtag. Inventory commands never modify configured targets; sm apply is the sole persistent projection operation. Isolated shells use stable generated link sets without changing global targets.\n\nsm does not run package installers, parse SKILL.md, or invoke Git. Successful mutations are silent unless --dry-run is used.",
    after_long_help = "Common workflows:\n  sm profiles new matt\n  sm import --profile matt --from ~/.agents/skills --create\n  sm update --from ~/.agents/skills\n  sm apply --force\n  sm shell --profile matt\n\nRun 'sm <command> --help' for command-specific behavior and examples.",
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
    /// List profiles or create new profiles.
    #[command(
        long_about = "List profiles or create new profiles.\n\nWith no subcommand, profile names are printed in bytewise lexical order. A profile is an immediate non-hidden directory under $SM_HOME/profiles. Missing .smtag files are initialized as enabled.",
        after_long_help = "Examples:\n  sm profiles\n  sm profiles new matt\n  sm profiles new archive --disabled"
    )]
    Profiles {
        #[command(subcommand)]
        command: Option<ProfilesCommand>,
    },

    /// List profile-qualified skills.
    #[command(
        long_about = "List profile-qualified skills.\n\nEach line has the form <profile>/<skill>. With no PROFILE arguments, every profile is inspected. sm identifies skills by immediate directory name and does not inspect SKILL.md.",
        after_long_help = "Examples:\n  sm skills\n  sm skills matt research"
    )]
    Skills {
        #[arg(value_name = "PROFILE")]
        profiles: Vec<String>,
    },

    /// List configured targets.
    #[command(
        long_about = "List configured targets.\n\nTargets are projection destinations with optional shell adapters. They do not own profile activation state.",
        after_long_help = "Examples:\n  sm targets"
    )]
    Targets,

    /// List globally enabled profiles.
    #[command(
        long_about = "List globally enabled profiles.\n\nActivation is read from each profile's .smtag and is shared by every target.",
        after_long_help = "Examples:\n  sm enabled"
    )]
    Enabled,

    /// List materialized managed links.
    #[command(
        long_about = "List materialized managed links.\n\nWithout --target, each tab-separated line contains target, skill, source, and destination. With --target, the target column is omitted.",
        after_long_help = "Examples:\n  sm status\n  sm status --target agents"
    )]
    Status {
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
    },

    /// Globally enable profiles.
    #[command(
        long_about = "Globally enable profiles by writing true to their .smtag files.\n\nEnabled profiles may not contain duplicate skill names. This command changes inventory state only; run sm apply to update targets.",
        after_long_help = "Examples:\n  sm enable common matt\n  sm enable matt --dry-run"
    )]
    Enable {
        #[arg(required = true, value_name = "PROFILE")]
        profiles: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },

    /// Globally disable profiles.
    #[command(
        long_about = "Globally disable profiles by writing false to their .smtag files.\n\nThis command changes inventory state only; run sm apply to update targets.",
        after_long_help = "Examples:\n  sm disable archive\n  sm disable matt --dry-run"
    )]
    Disable {
        #[arg(required = true, value_name = "PROFILE")]
        profiles: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },

    /// Reconcile configured targets with globally enabled profiles.
    #[command(
        long_about = "Reconcile configured targets with globally enabled profiles.\n\nBy default every persistent target is preflighted before any is changed. Real directories block reconciliation unless --force removes them. Ordinary files, hidden entries, and unrelated symlinks are preserved unless they block a desired skill name.",
        after_long_help = "Examples:\n  sm apply\n  sm apply --force\n  sm apply --target agents --dry-run"
    )]
    Apply {
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
        #[arg(short, long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },

    /// Start an isolated child shell.
    #[command(
        long_about = "Start an isolated zsh, Bash, Fish, or other child shell.\n\nConfigured shell adapters are wrapped to use one stable generation. Without --target every available adapter is wrapped. Any --profile makes the profile set explicit; otherwise globally enabled profiles are inherited.",
        after_long_help = "Examples:\n  sm shell\n  sm shell --profile matt\n  sm shell --target pi --skill research/web-search\n  sm shell -- --no-rcs"
    )]
    Shell {
        /// Deprecated positional target; prefer --target.
        #[arg(value_name = "TARGET", hide = true, conflicts_with = "targets")]
        target: Option<String>,
        #[arg(short, long = "target", value_name = "TARGET")]
        targets: Vec<String>,
        #[arg(long = "profile", value_name = "PROFILE")]
        profiles: Vec<String>,
        #[arg(long = "skill", value_name = "PROFILE/SKILL")]
        skills: Vec<String>,
        #[arg(last = true, value_name = "SHELL_ARGUMENT")]
        shell_args: Vec<OsString>,
    },

    /// Copy profiles or skills into a project directory.
    #[command(
        long_about = "Copy profiles or individual skills into a project-owned directory.\n\nAt least one selector is required. Existing entries are never overwritten and the copies are not tracked afterward.",
        after_long_help = "Examples:\n  sm export --profile common\n  sm export --skill research/web-search --to .agents/skills"
    )]
    Export {
        #[arg(long = "profile", value_name = "PROFILE")]
        profiles: Vec<String>,
        #[arg(long = "skill", value_name = "PROFILE/SKILL")]
        skills: Vec<String>,
        #[arg(long = "to", default_value = ".skills", value_name = "DIRECTORY")]
        destination: OsString,
        #[arg(long)]
        dry_run: bool,
    },

    /// Copy source skills into one profile.
    #[command(
        long_about = "Copy source skills into one profile.\n\nImport may add skills but never removes source directories. Managed sm links in the source are skipped. The profile must exist unless --create is supplied, and existing destination skills require --replace. Run sm apply separately.",
        after_long_help = "Examples:\n  sm import --profile matt --from ~/.agents/skills --create\n  sm import --profile matt --skill code-review --replace"
    )]
    Import {
        #[arg(long, value_name = "PROFILE")]
        profile: String,
        #[arg(long = "skill", value_name = "SKILL")]
        skills: Vec<String>,
        #[arg(long = "from", default_value = ".skills", value_name = "DIRECTORY")]
        source: OsString,
        #[arg(long)]
        create: bool,
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        dry_run: bool,
    },

    /// Replace existing inventory skills from same-named source directories.
    #[command(
        long_about = "Replace existing inventory skills from same-named source directories.\n\nUpdate never creates a profile or skill. Unknown source directories are skipped. Duplicate inventory names require --profile to select one owner or --all to update every existing copy. Run sm apply separately.",
        after_long_help = "Examples:\n  sm update --from ~/.agents/skills\n  sm update --from ~/.agents/skills --profile matt\n  sm update --from ~/.agents/skills --all --dry-run"
    )]
    Update {
        #[arg(long = "from", value_name = "DIRECTORY", required = true)]
        source: OsString,
        #[arg(long = "skill", value_name = "SKILL")]
        skills: Vec<String>,
        #[arg(long, value_name = "PROFILE", conflicts_with = "all")]
        profile: Option<String>,
        #[arg(long, conflicts_with = "profile")]
        all: bool,
        #[arg(long)]
        dry_run: bool,
    },

    /// Register a skill directory as a target.
    #[command(
        long_about = "Register a skill directory as a persistent target.\n\nAdopt edits config.toml while preserving existing formatting and comments. It does not move skills, infer a shell adapter, or run sm apply.",
        after_long_help = "Examples:\n  sm adopt agents ~/.agents/skills\n  sm adopt pi ~/.pi/agent/skills --dry-run"
    )]
    Adopt {
        #[arg(value_name = "TARGET")]
        target: String,
        #[arg(value_name = "DIRECTORY")]
        directory: OsString,
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove shell generations without live leases.
    #[command(
        long_about = "Remove stable shell generations that have no live process lease.\n\nRunning child shells and wrapped commands are skipped. Unknown cache entries are preserved.",
        after_long_help = "Examples:\n  sm gc --dry-run\n  sm gc"
    )]
    Gc {
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

#[derive(Debug, Subcommand)]
pub enum ProfilesCommand {
    /// Create one or more profiles.
    New {
        #[arg(required = true, value_name = "PROFILE")]
        profiles: Vec<String>,
        #[arg(long)]
        disabled: bool,
        #[arg(long)]
        dry_run: bool,
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
