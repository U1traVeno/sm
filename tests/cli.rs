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
        let path = self.sm_home.join("profiles").join(profile).join(skill);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("origin"), content).unwrap();
        path
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
fn enable_precedence_disable_fallback_and_apply() {
    let fixture = Fixture::new();
    let common = fixture
        .add_skill("common", "shared", "common\n")
        .canonicalize()
        .unwrap();
    let coding = fixture
        .add_skill("coding", "shared", "coding\n")
        .canonicalize()
        .unwrap();
    fixture.add_skill("coding", "rust", "rust\n");
    fixture.write_config(None);

    let output = fixture.run(&["enable", "common", "coding"]);
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(fixture.run(&["enabled"]).stdout).unwrap(),
        "common\ncoding\n"
    );
    assert_eq!(
        fs::read_link(fixture.target.join("shared")).unwrap(),
        coding
    );
    assert!(fixture.target.join("rust").is_symlink());

    fixture.run(&["disable", "coding"]);
    assert_eq!(
        fs::read_link(fixture.target.join("shared")).unwrap(),
        common
    );
    assert!(!fixture.target.join("rust").exists());

    fs::remove_file(fixture.target.join("shared")).unwrap();
    fixture.run(&["apply"]);
    assert_eq!(
        fs::read_link(fixture.target.join("shared")).unwrap(),
        common
    );

    let markers = fs::read_dir(fixture.state_home.join("sm/targets/fake/enabled"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 1);
    assert!(markers[0].ends_with("-common"));
}

#[test]
fn help_is_complete_for_every_public_command() {
    let fixture = Fixture::new();
    let top = fixture.command().arg("--help").output().unwrap();
    assert!(top.status.success());
    let top = String::from_utf8(top.stdout).unwrap();
    for expected in [
        "sm does not install skill packages",
        "Common workflows:",
        "Locations:",
        "sm <command> --help",
    ] {
        assert!(
            top.contains(expected),
            "top-level help is missing {expected:?}"
        );
    }

    let commands = [
        ("profiles", "bytewise lexical order"),
        ("skills", "<profile>/<skill>"),
        ("targets", "config.toml"),
        ("enabled", "lowest to highest precedence"),
        ("status", "tab-separated"),
        ("enable", "processed from left to right"),
        ("disable", "idempotent success"),
        ("apply", "without changing enabled profiles"),
        ("shell", "Without --profile"),
        ("export", "one-time ownership transfer"),
        ("import", "one-time ownership transfer"),
        ("gc", "PID-based directory leases"),
    ];
    for (command, expected) in commands {
        let output = fixture
            .command()
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{command} --help failed");
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("Usage:"), "{command} help has no usage");
        assert!(help.contains("Examples:"), "{command} help has no examples");
        assert!(
            help.contains(expected),
            "{command} help is missing {expected:?}\n{help}"
        );
    }
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
fn dry_run_is_non_mutating_and_stale_directory_lock_recovers() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "shared", "common\n");
    fixture.write_config(None);

    let output = fixture.run(&["enable", "common", "--dry-run"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("enable\tfake\tcommon"));
    assert!(!fixture.target.exists());
    assert!(!fixture.state_home.join("sm/targets/fake").exists());

    let stale_lock = fixture.state_home.join("sm/targets/fake/lock");
    fs::create_dir_all(stale_lock.join("owner-99999999")).unwrap();
    fixture.run(&["enable", "common"]);
    assert!(fixture.target.join("shared").is_symlink());
    assert!(!stale_lock.exists());
}

#[test]
fn unmanaged_collision_fails_before_enabled_state_is_saved() {
    let fixture = Fixture::new();
    fixture.add_skill("common", "blocked", "managed\n");
    fixture.write_config(None);
    fs::create_dir_all(fixture.target.join("blocked")).unwrap();
    fs::write(fixture.target.join("blocked/user"), "user\n").unwrap();

    let output = fixture
        .command()
        .args(["enable", "common"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unmanaged target entry"));
    assert!(!fixture.state_home.join("sm/targets/fake/enabled").exists());
    assert_eq!(
        fs::read_to_string(fixture.target.join("blocked/user")).unwrap(),
        "user\n"
    );

    fs::remove_dir_all(fixture.target.join("blocked")).unwrap();
    let profile_root_link = fixture.target.join("profile-root");
    symlink(fixture.sm_home.join("profiles/common"), &profile_root_link).unwrap();
    fixture.run(&["enable", "common"]);
    assert!(profile_root_link.is_symlink());
}

#[test]
fn export_and_import_transfer_ownership_without_overwrite() {
    let fixture = Fixture::new();
    fixture.add_skill("coding", "rust", "rust\n");
    fixture.add_skill("coding", "shared", "coding\n");
    fixture.add_skill("common", "shared", "common\n");
    fixture.write_config(None);

    fixture.run(&["export", "--profile", "coding", "--skill", "common/shared"]);
    assert_eq!(
        fs::read_to_string(fixture.project.join(".skills/shared/origin")).unwrap(),
        "common\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.project.join(".skills/rust/origin")).unwrap(),
        "rust\n"
    );

    fixture.run(&["import", "--profile", "imported", "--skill", "rust"]);
    assert_eq!(
        fs::read_to_string(fixture.sm_home.join("profiles/imported/rust/origin")).unwrap(),
        "rust\n"
    );

    let output = fixture
        .command()
        .args(["import", "--profile", "imported", "--skill", "rust"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));

    fs::create_dir_all(fixture.sm_home.join("profiles/empty")).unwrap();
    fs::remove_dir_all(fixture.project.join(".skills")).unwrap();
    fixture.run(&["export", "--profile", "empty"]);
    assert!(fixture.project.join(".skills").is_dir());

    let output = fixture.command().arg("export").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires at least one"));
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
