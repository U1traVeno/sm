#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
    home: PathBuf,
    sm_home: PathBuf,
    config_home: PathBuf,
    state_home: PathBuf,
    cache_home: PathBuf,
    target: PathBuf,
    project: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let sm_home = root.path().join("sm-home");
        let config_home = root.path().join("config");
        let state_home = root.path().join("state");
        let cache_home = root.path().join("cache");
        let target = root.path().join("target");
        let project = root.path().join("project");
        for path in [
            &home,
            &sm_home,
            &config_home,
            &state_home,
            &cache_home,
            &project,
        ] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            _root: root,
            home,
            sm_home,
            config_home,
            state_home,
            cache_home,
            target,
            project,
        }
    }

    fn add_skill(&self, profile: &str, skill: &str, content: &str) -> PathBuf {
        let profile_path = self.sm_home.join("profiles").join(profile);
        let path = profile_path.join(skill);
        fs::create_dir_all(&path).unwrap();
        if !profile_path.join(".smtag").exists() {
            fs::write(profile_path.join(".smtag"), "true\n").unwrap();
        }
        fs::write(path.join("origin"), content).unwrap();
        path
    }

    fn set_enabled(&self, profile: &str, enabled: bool) {
        let path = self.sm_home.join("profiles").join(profile);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join(".smtag"),
            if enabled { "true\n" } else { "false\n" },
        )
        .unwrap();
    }

    fn write_config(&self, shell: Option<(&Path, &Path)>) {
        let config_dir = self.config_home.join("sm");
        fs::create_dir_all(&config_dir).unwrap();
        let mut config = format!(
            "default_target = \"fake\"\n\n[targets.fake]\nskills_dir = {:?}\n",
            self.target.to_string_lossy()
        );
        if let Some((agent, output)) = shell {
            config.push_str(&format!(
                "\n[targets.fake.shell]\ncommand = {:?}\nargs = [\"--skills\", \"{{skills}}\"]\nenv = {{ TEST_SCOPE = \"{{skills}}\", OUTPUT = {:?} }}\n",
                agent.to_string_lossy(),
                output.to_string_lossy()
            ));
        }
        fs::write(config_dir.join("config.toml"), config).unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sm"));
        command
            .env("HOME", &self.home)
            .env("SM_HOME", &self.sm_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("SHELL", "/bin/sh")
            .env("PATH", "/usr/bin:/bin")
            .current_dir(&self.project);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        let output = self.command().args(args).output().unwrap();
        assert!(
            output.status.success(),
            "sm {args:?} failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

#[test]
fn global_tags_are_initialized_and_inventory_changes_require_apply() {
    let fixture = Fixture::new();
    let skill = fixture.sm_home.join("profiles/common/shared");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("origin"), "common\n").unwrap();
    let legacy = fixture
        .state_home
        .join("sm/targets/fake/enabled/0001-common");
    fs::create_dir_all(&legacy).unwrap();
    fixture.write_config(None);

    let enabled = fixture.run(&["enabled"]);
    assert_eq!(String::from_utf8(enabled.stdout).unwrap(), "common\n");
    let diagnostics = String::from_utf8(enabled.stderr).unwrap();
    assert!(diagnostics.contains("initialized profile common as enabled"));
    assert!(diagnostics.contains("removed obsolete per-target activation state"));
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/common/.smtag")).unwrap(),
        "true\n"
    );
    assert!(!legacy.exists());

    fixture.run(&["disable", "common"]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/common/.smtag")).unwrap(),
        "false\n"
    );
    fixture.run(&["apply"]);
    assert!(!fixture.target.join("shared").exists());

    fixture.run(&["enable", "common"]);
    assert!(!fixture.target.join("shared").exists());
    fixture.run(&["apply"]);
    assert_eq!(
        fs::read_link(fixture.target.join("shared")).unwrap(),
        skill.canonicalize().unwrap()
    );

    fs::remove_dir_all(fixture.sm_home.join("profiles/common")).unwrap();
    fixture.run(&["apply"]);
    assert!(!fixture.target.join("shared").exists());
}

#[test]
fn enabled_profiles_reject_duplicate_skill_names_without_precedence() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    let coding = fixture.add_skill("coding", "shared", "coding\n");
    fixture.set_enabled("coding", false);
    fixture.write_config(None);

    let output = fixture
        .command()
        .args(["enable", "coding"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate skill shared"));
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/coding/.smtag")).unwrap(),
        "false\n"
    );

    fixture.run(&["disable", "common"]);
    fixture.run(&["enable", "coding"]);
    fixture.run(&["apply"]);
    assert_eq!(
        fs::read_link(fixture.target.join("shared")).unwrap(),
        coding.canonicalize().unwrap()
    );
}

#[test]
fn apply_force_removes_real_directories_and_preserves_other_unmanaged_entries() {
    let fixture = Fixture::new();
    let managed = fixture.add_skill("common", "managed", "canonical\n");
    fixture.write_config(None);
    fs::create_dir_all(fixture.target.join("managed")).unwrap();
    fs::create_dir_all(fixture.target.join("unwanted")).unwrap();
    fs::create_dir_all(fixture.target.join(".hidden")).unwrap();
    fs::write(fixture.target.join("notes.txt"), "keep\n").unwrap();
    let external = fixture.project.join("external");
    fs::create_dir_all(&external).unwrap();
    symlink(&external, fixture.target.join("custom-link")).unwrap();

    let output = fixture.command().arg("apply").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("rerun with --force"));
    assert!(fixture.target.join("managed").is_dir());
    assert!(fixture.target.join("unwanted").is_dir());

    fixture.run(&["apply", "--force"]);
    assert_eq!(
        fs::read_link(fixture.target.join("managed")).unwrap(),
        managed.canonicalize().unwrap()
    );
    assert!(!fixture.target.join("unwanted").exists());
    assert!(fixture.target.join(".hidden").is_dir());
    assert_eq!(
        fs::read_to_string(fixture.target.join("notes.txt")).unwrap(),
        "keep\n"
    );
    assert!(fixture.target.join("custom-link").is_symlink());
}

#[test]
fn apply_rejects_targets_that_overlap_the_profile_inventory() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "managed", "canonical\n");
    let config_dir = fixture.config_home.join("sm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        format!(
            "[targets.dangerous]\nskills_dir = {:?}\n",
            fixture.sm_home.join("profiles").to_string_lossy()
        ),
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["apply", "--force"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("overlaps the profile inventory"));
    assert!(fixture.sm_home.join("profiles/common/managed").is_dir());
}

#[test]
fn apply_preflights_every_target_before_mutation_and_status_lists_all() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    let second = fixture._root.path().join("second-target");
    fs::create_dir_all(second.join("blocked")).unwrap();
    let config_dir = fixture.config_home.join("sm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        format!(
            "[targets.a]\nskills_dir = {:?}\n\n[targets.b]\nskills_dir = {:?}\n",
            fixture.target.to_string_lossy(),
            second.to_string_lossy()
        ),
    )
    .unwrap();

    let output = fixture.command().arg("apply").output().unwrap();
    assert!(!output.status.success());
    assert!(!fixture.target.exists());

    fixture.run(&["apply", "--force"]);
    assert!(fixture.target.join("shared").is_symlink());
    assert!(second.join("shared").is_symlink());
    assert!(!second.join("blocked").exists());
    let status = String::from_utf8(fixture.run(&["status"]).stdout).unwrap();
    assert!(status.lines().any(|line| line.starts_with("a\tshared\t")));
    assert!(status.lines().any(|line| line.starts_with("b\tshared\t")));
}

#[test]
fn import_can_create_inventory_but_leaves_sources_and_targets_unchanged() {
    let fixture = Fixture::new();
    let source = fixture.project.join("incoming");
    fs::create_dir_all(source.join("rust")).unwrap();
    fs::write(source.join("rust/origin"), "rust\n").unwrap();
    let existing = fixture.add_skill("common", "managed", "managed\n");
    symlink(&existing, source.join("managed")).unwrap();
    fixture.write_config(None);

    let output = fixture
        .command()
        .args(["import", "--profile", "matt", "--from", "incoming"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--create"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    fixture.run(&[
        "import",
        "--profile",
        "matt",
        "--from",
        "incoming",
        "--create",
    ]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/matt/rust/origin")).unwrap(),
        "rust\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/matt/.smtag")).unwrap(),
        "true\n"
    );
    assert!(source.join("rust").is_dir());
    assert!(source.join("managed").is_symlink());
    assert!(!fixture.target.exists());
}

#[test]
fn update_replaces_only_existing_skills_and_requires_duplicate_disambiguation() {
    let fixture = Fixture::new();
    fixture.add_skill("one", "shared", "one-old\n");
    fixture.add_skill("two", "shared", "two-old\n");
    fixture.set_enabled("two", false);
    fixture.add_skill("one", "only-one", "old\n");
    let source = fixture.project.join("download");
    for (name, content) in [
        ("shared", "new-shared\n"),
        ("only-one", "new-one\n"),
        ("unknown", "unknown\n"),
    ] {
        fs::create_dir_all(source.join(name)).unwrap();
        fs::write(source.join(name).join("origin"), content).unwrap();
    }

    let output = fixture
        .command()
        .args(["update", "--from", "download"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("use --profile or --all"));
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/one/shared/origin")).unwrap(),
        "one-old\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/one/only-one/origin")).unwrap(),
        "old\n"
    );

    fixture.run(&["update", "--from", "download", "--profile", "one"]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/one/shared/origin")).unwrap(),
        "new-shared\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/one/only-one/origin")).unwrap(),
        "new-one\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/two/shared/origin")).unwrap(),
        "two-old\n"
    );
    assert!(!fixture.sm_home.join("profiles/one/unknown").exists());
    assert!(source.join("unknown").is_dir());

    fixture.run(&["update", "--from", "download", "--skill", "shared", "--all"]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/two/shared/origin")).unwrap(),
        "new-shared\n"
    );
}

#[test]
fn update_ignores_non_directory_entries_in_the_source_root() {
    let fixture = Fixture::new();
    fixture.add_skill("derivon", "derivon-cli", "old\n");
    let source = fixture.project.join("source");
    fs::create_dir_all(source.join("derivon-cli")).unwrap();
    fs::write(source.join("derivon-cli/origin"), "new\n").unwrap();
    fs::write(source.join("CONTEXT.md"), "context\n").unwrap();
    fs::write(source.join("package.json"), "{}\n").unwrap();

    fixture.run(&["update", "--from", "source"]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/derivon/derivon-cli/origin")).unwrap(),
        "new\n"
    );
}

#[test]
fn adopt_preserves_config_and_is_idempotent() {
    let fixture = Fixture::new();
    let config = fixture.config_home.join("sm/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        &config,
        "# keep this comment\n[targets.pi]\n\n[targets.pi.shell]\ncommand = \"pi\"\nargs = [\"--skill\", \"{skills}\"]\n",
    )
    .unwrap();
    let adopted = fixture._root.path().join("not-created-yet");
    let adopted_text = adopted.to_string_lossy().into_owned();

    let dry = fixture.run(&["adopt", "pi", &adopted_text, "--dry-run"]);
    assert!(String::from_utf8_lossy(&dry.stdout).contains("adopt\tpi"));
    assert!(!fs::read_to_string(&config).unwrap().contains("skills_dir"));

    fixture.run(&["adopt", "pi", &adopted_text]);
    fixture.run(&["adopt", "pi", &adopted_text]);
    let text = fs::read_to_string(&config).unwrap();
    assert!(text.contains("# keep this comment"));
    assert!(text.contains("skills_dir"));
    assert!(text.contains("[targets.pi.shell]"));
    assert!(!adopted.exists());

    let output = fixture
        .command()
        .args(["adopt", "other", &adopted_text])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already adopted"));
}

#[test]
fn dry_run_does_not_initialize_tags_or_remove_legacy_state() {
    let fixture = Fixture::new();
    let profile = fixture.sm_home.join("profiles/common");
    fs::create_dir_all(&profile).unwrap();
    let legacy = fixture
        .state_home
        .join("sm/targets/fake/enabled/0001-common");
    fs::create_dir_all(&legacy).unwrap();
    fixture.write_config(None);

    let output = fixture.run(&["apply", "--dry-run"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("tag\ttrue"));
    assert!(stdout.contains("remove\t"));
    assert!(!profile.join(".smtag").exists());
    assert!(legacy.exists());
    assert!(!fixture.target.exists());

    let stale_lock = fixture.state_home.join("sm/inventory/lock/owner-99999999");
    fs::create_dir_all(&stale_lock).unwrap();
    fixture.run(&["profiles"]);
    assert!(!fixture.state_home.join("sm/inventory/lock").exists());
}

#[test]
fn failed_staged_update_preserves_existing_inventory() {
    let fixture = Fixture::new();
    fixture.add_skill("matt", "rust", "old\n");
    let source = fixture.project.join("broken/rust");
    fs::create_dir_all(&source).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(source.join("socket")).unwrap();

    let output = fixture
        .command()
        .args(["update", "--from", "broken"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported file type"));
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/matt/rust/origin")).unwrap(),
        "old\n"
    );
    assert_eq!(
        fs::read_dir(fixture.sm_home.join("profiles"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".sm-install"))
            .count(),
        0
    );
}

#[test]
fn help_is_complete_for_every_public_command() {
    let fixture = Fixture::new();
    let top = String::from_utf8(fixture.command().arg("--help").output().unwrap().stdout).unwrap();
    for expected in [
        "does not run package installers",
        "Common workflows:",
        "sm <command> --help",
    ] {
        assert!(
            top.contains(expected),
            "top-level help is missing {expected:?}"
        );
    }
    for command in [
        "profiles", "skills", "targets", "enabled", "status", "enable", "disable", "apply",
        "shell", "export", "import", "update", "adopt", "gc",
    ] {
        let output = fixture
            .command()
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{command} --help failed");
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("Usage:"), "{command} help has no usage");
        assert!(help.contains("Examples:"), "{command} help has no examples");
    }
    let new_help = fixture
        .command()
        .args(["profiles", "new", "--help"])
        .output()
        .unwrap();
    assert!(new_help.status.success());
}

#[test]
fn target_independent_commands_ignore_broken_target_config() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    let config_dir = fixture.config_home.join("sm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "this is not toml = [").unwrap();

    assert_eq!(
        String::from_utf8(fixture.run(&["profiles"]).stdout).unwrap(),
        "common\n"
    );
    fixture.run(&["export", "--profile", "common"]);
    assert!(fixture.project.join(".skills/shared").is_dir());
    fixture.run(&["gc"]);

    let output = fixture
        .command()
        .args(["__exec", "../../outside", "--", "/bin/true"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid generation ID"));
}

#[test]
fn zsh_startup_files_cannot_shadow_the_generated_wrapper() {
    let Some(zsh) = [Path::new("/bin/zsh"), Path::new("/usr/bin/zsh")]
        .into_iter()
        .find(|path| path.is_file())
    else {
        return;
    };

    let fixture = Fixture::new();
    fixture.add_skill("coding", "rust", "rust\n");

    let agent = fixture.home.join("fake-agent");
    let output_prefix = fixture.home.join("agent-output");
    fs::write(
        &agent,
        "#!/bin/sh\nprintf 'real\\n' > \"$OUTPUT.kind\"\nprintf '%s\\n' \"$STARTUP_ENV\" \"$STARTUP_RC\" \"$ZDOTDIR\" > \"$OUTPUT.startup\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&agent, permissions).unwrap();
    fixture.write_config(Some((&agent, &output_prefix)));

    let shadow_dir = fixture.home.join("shadow-bin");
    fs::create_dir_all(&shadow_dir).unwrap();
    let shadow_agent = shadow_dir.join("fake-agent");
    let shadow_output = fixture.home.join("shadow-output");
    fs::write(
        &shadow_agent,
        format!(
            "#!/bin/sh\nprintf 'shadow\\n' > {:?}\n",
            shadow_output.to_string_lossy()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&shadow_agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&shadow_agent, permissions).unwrap();

    let zdotdir = fixture.home.join("zsh-config");
    fs::create_dir_all(&zdotdir).unwrap();
    fs::write(zdotdir.join(".zshenv"), "export STARTUP_ENV=loaded\n").unwrap();
    fs::write(
        zdotdir.join(".zshrc"),
        format!(
            "export PATH={:?}:$PATH\nexport STARTUP_RC=loaded\n",
            shadow_dir.to_string_lossy()
        ),
    )
    .unwrap();

    let output = fixture
        .command()
        .env("SHELL", zsh)
        .env("ZDOTDIR", &zdotdir)
        .args([
            "shell",
            "fake",
            "--profile",
            "coding",
            "--",
            "-ic",
            "fake-agent",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "zsh shell failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("kind")).unwrap(),
        "real\n"
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("startup")).unwrap(),
        format!("loaded\nloaded\n{}\n", zdotdir.display())
    );
    assert!(!shadow_output.exists());
}

#[test]
fn bash_startup_files_cannot_shadow_the_generated_wrapper() {
    let fixture = Fixture::new();
    fixture.add_skill("coding", "rust", "rust\n");

    let agent = fixture.home.join("fake-agent");
    let output_prefix = fixture.home.join("agent-output");
    fs::write(
        &agent,
        "#!/bin/sh\nprintf 'real\\n' > \"$OUTPUT.kind\"\nprintf '%s\\n' \"$STARTUP_BASHRC\" \"$STARTUP_BASH_ENV\" \"${BASH_ENV-<unset>}\" > \"$OUTPUT.startup\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&agent, permissions).unwrap();
    fixture.write_config(Some((&agent, &output_prefix)));

    let shadow_dir = fixture.home.join("shadow-bin");
    fs::create_dir_all(&shadow_dir).unwrap();
    let shadow_agent = shadow_dir.join("fake-agent");
    let shadow_output = fixture.home.join("shadow-output");
    fs::write(
        &shadow_agent,
        format!(
            "#!/bin/sh\nprintf 'shadow\\n' > {:?}\n",
            shadow_output.to_string_lossy()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&shadow_agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&shadow_agent, permissions).unwrap();

    fs::write(
        fixture.home.join(".bashrc"),
        format!(
            "export PATH={:?}:$PATH\nexport STARTUP_BASHRC=loaded\n",
            shadow_dir.to_string_lossy()
        ),
    )
    .unwrap();
    let bash_env = fixture.home.join("user-bash-env");
    fs::write(
        &bash_env,
        format!(
            "export PATH={:?}:$PATH\nexport STARTUP_BASH_ENV=loaded\n",
            shadow_dir.to_string_lossy()
        ),
    )
    .unwrap();

    let interactive = fixture
        .command()
        .env("SHELL", "/bin/bash")
        .env("BASH_ENV", &bash_env)
        .args([
            "shell",
            "fake",
            "--profile",
            "coding",
            "--",
            "-ic",
            "fake-agent",
        ])
        .output()
        .unwrap();
    assert!(
        interactive.status.success(),
        "interactive bash failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&interactive.stdout),
        String::from_utf8_lossy(&interactive.stderr)
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("kind")).unwrap(),
        "real\n"
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("startup")).unwrap(),
        format!("loaded\n\n{}\n", bash_env.display())
    );
    assert!(!shadow_output.exists());

    fs::remove_file(output_prefix.with_extension("kind")).unwrap();
    fs::remove_file(output_prefix.with_extension("startup")).unwrap();
    let non_interactive = fixture
        .command()
        .env("SHELL", "/bin/bash")
        .env("BASH_ENV", &bash_env)
        .args([
            "shell",
            "fake",
            "--profile",
            "coding",
            "--",
            "-c",
            "fake-agent",
        ])
        .output()
        .unwrap();
    assert!(
        non_interactive.status.success(),
        "non-interactive bash failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&non_interactive.stdout),
        String::from_utf8_lossy(&non_interactive.stderr)
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("kind")).unwrap(),
        "real\n"
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("startup")).unwrap(),
        format!("\nloaded\n{}\n", bash_env.display())
    );
    assert!(!shadow_output.exists());

    let login = fixture
        .command()
        .env("SHELL", "/bin/bash")
        .args(["shell", "fake", "--profile", "coding", "--", "-l"])
        .output()
        .unwrap();
    assert!(!login.status.success());
    assert!(
        String::from_utf8_lossy(&login.stderr)
            .contains("bash login shells cannot preserve wrapper precedence")
    );
}

#[test]
fn fish_config_cannot_shadow_the_generated_wrapper() {
    let fish = std::env::var_os("SM_TEST_FISH")
        .map(PathBuf::from)
        .or_else(|| {
            [
                Path::new("/opt/homebrew/bin/fish"),
                Path::new("/usr/bin/fish"),
            ]
            .into_iter()
            .find(|path| path.is_file())
            .map(Path::to_path_buf)
        });
    let Some(fish) = fish else {
        return;
    };

    let fixture = Fixture::new();
    fixture.add_skill("coding", "rust", "rust\n");

    let agent = fixture.home.join("fake-agent");
    let output_prefix = fixture.home.join("agent-output");
    fs::write(
        &agent,
        "#!/bin/sh\nprintf 'real\\n' > \"$OUTPUT.kind\"\nprintf '%s\\n' \"$STARTUP_FISH\" > \"$OUTPUT.startup\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&agent, permissions).unwrap();
    fixture.write_config(Some((&agent, &output_prefix)));

    let shadow_dir = fixture.home.join("shadow-bin");
    fs::create_dir_all(&shadow_dir).unwrap();
    let shadow_agent = shadow_dir.join("fake-agent");
    let shadow_output = fixture.home.join("shadow-output");
    fs::write(
        &shadow_agent,
        format!(
            "#!/bin/sh\nprintf 'shadow\\n' > {:?}\n",
            shadow_output.to_string_lossy()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&shadow_agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&shadow_agent, permissions).unwrap();

    let fish_config = fixture.config_home.join("fish");
    fs::create_dir_all(&fish_config).unwrap();
    fs::write(
        fish_config.join("config.fish"),
        format!(
            "set -gx PATH {:?} $PATH\nset -gx STARTUP_FISH loaded\n",
            shadow_dir.to_string_lossy()
        ),
    )
    .unwrap();

    let output = fixture
        .command()
        .env("SHELL", &fish)
        .args([
            "shell",
            "fake",
            "--profile",
            "coding",
            "--",
            "-ic",
            "fake-agent",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fish shell failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("kind")).unwrap(),
        "real\n"
    );
    assert_eq!(
        fs::read_to_string(output_prefix.with_extension("startup")).unwrap(),
        "loaded\n"
    );
    assert!(!shadow_output.exists());
}

#[test]
fn shell_uses_generic_template_and_isolated_generation() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    fixture.add_skill("coding", "rust", "rust\n");

    let agent = fixture.home.join("fake-agent");
    let output_prefix = fixture.home.join("agent-output");
    fs::write(
        &agent,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$OUTPUT.args\"\nprintf '%s\\n' \"$SM_SKILLS_DIR\" \"$TEST_SCOPE\" > \"$OUTPUT.env\"\nfind \"$SM_SKILLS_DIR\" -mindepth 1 -maxdepth 1 -type l -exec basename {} \\; | sort > \"$OUTPUT.skills\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&agent).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&agent, permissions).unwrap();
    fixture.write_config(Some((&agent, &output_prefix)));
    fixture.run(&["enable", "common"]);

    fixture.run(&[
        "shell",
        "fake",
        "--skill",
        "coding/rust",
        "--",
        "-c",
        "fake-agent user-arg",
    ]);

    let args = fs::read_to_string(output_prefix.with_extension("args")).unwrap();
    let env = fs::read_to_string(output_prefix.with_extension("env")).unwrap();
    let skills = fs::read_to_string(output_prefix.with_extension("skills")).unwrap();
    let arg_lines = args.lines().collect::<Vec<_>>();
    assert_eq!(arg_lines[0], "--skills");
    assert!(arg_lines[1].ends_with("/skills"));
    assert_eq!(arg_lines[2], "user-arg");
    let env_lines = env.lines().collect::<Vec<_>>();
    assert_eq!(env_lines[0], env_lines[1]);
    assert_eq!(env_lines[0], arg_lines[1]);
    assert_eq!(skills, "rust\nshared\n");

    fixture.run(&[
        "shell",
        "fake",
        "--profile",
        "coding",
        "--",
        "-c",
        "fake-agent explicit",
    ]);
    let explicit_skills = fs::read_to_string(output_prefix.with_extension("skills")).unwrap();
    assert_eq!(explicit_skills, "rust\n");

    let unmanaged_cache = fixture.cache_home.join("sm/generations/user-data");
    fs::create_dir_all(&unmanaged_cache).unwrap();
    fs::write(unmanaged_cache.join("keep"), "user\n").unwrap();

    let dry_run = String::from_utf8(fixture.run(&["gc", "--dry-run"]).stdout).unwrap();
    assert!(dry_run.starts_with("remove\t"));
    fixture.run(&["gc"]);
    assert_eq!(
        fs::read_dir(fixture.cache_home.join("sm/generations"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        fs::read_to_string(unmanaged_cache.join("keep")).unwrap(),
        "user\n"
    );
}

#[test]
fn shell_wraps_every_configured_adapter_with_one_generation() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    let agent_a = fixture.home.join("agent-a");
    let agent_b = fixture.home.join("agent-b");
    let output_a = fixture.home.join("agent-a-output");
    let output_b = fixture.home.join("agent-b-output");
    for agent in [&agent_a, &agent_b] {
        fs::write(
            agent,
            "#!/bin/sh\nprintf '%s\\n' \"$1\" > \"$OUTPUT.path\"\nfind \"$1\" -mindepth 1 -maxdepth 1 -type l -exec basename {} \\; | sort > \"$OUTPUT.skills\"\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(agent).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(agent, permissions).unwrap();
    }
    let config_dir = fixture.config_home.join("sm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        format!(
            "[targets.a.shell]\ncommand = {:?}\nargs = [\"{{skills}}\"]\nenv = {{ OUTPUT = {:?} }}\n\n[targets.b.shell]\ncommand = {:?}\nargs = [\"{{skills}}\"]\nenv = {{ OUTPUT = {:?} }}\n",
            agent_a.to_string_lossy(),
            output_a.to_string_lossy(),
            agent_b.to_string_lossy(),
            output_b.to_string_lossy(),
        ),
    )
    .unwrap();

    fixture.run(&["shell", "--", "-c", "agent-a && agent-b"]);
    let path_a = fs::read_to_string(output_a.with_extension("path")).unwrap();
    let path_b = fs::read_to_string(output_b.with_extension("path")).unwrap();
    assert_eq!(path_a, path_b);
    assert_eq!(
        fs::read_to_string(output_a.with_extension("skills")).unwrap(),
        "shared\n"
    );
    assert_eq!(
        fs::read_to_string(output_b.with_extension("skills")).unwrap(),
        "shared\n"
    );
}
