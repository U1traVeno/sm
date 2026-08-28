# Targets and Templates

Language: **English** | [简体中文](zh-cn/targets.md)

A target describes two optional capabilities:

1. A persistent skill directory used by `sm enable`, `sm disable`, and `sm apply`.
2. A generic command template used by `sm shell`.

The core does not contain agent-specific branches. A Pi target, a future agent target, and a user-defined command all use the same fields.

## Configuration File

The default configuration path is:

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

Example:

```toml
default_target = "pi"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]

[targets.example]
skills_dir = "~/.example/skills"

[targets.example.shell]
command = "/usr/local/bin/example-agent"
args = ["--skills-dir", "{skills}"]
env = { EXAMPLE_MODE = "isolated" }
```

## Target Selection

Commands select a target in this order:

1. `--target <name>`
2. `SM_TARGET`
3. `default_target` in `config.toml`
4. the only configured target, when exactly one exists

If selection remains ambiguous, the command fails and lists the available target names on standard error.

`sm shell <target>` always takes its target name explicitly.

## Persistent Capability

`skills_dir` is the directory where persistent activation links are materialized. `~` is expanded to the user's home directory. Relative paths are rejected.

A target without `skills_dir` cannot be used with:

- `enable`
- `disable`
- `apply`
- `status`

## Shell Capability

The shell table accepts:

- `command`: executable name or absolute path;
- `args`: argv items inserted before arguments entered by the user;
- `env`: optional environment variables applied by the wrapper.

`{skills}` expands to the absolute `skills/` directory of the stable generation. It must appear in at least one `args` or `env` value.

The template is data, not a shell command. `sm` performs no shell interpolation, word splitting, command substitution, or `eval`.

Before starting the child shell, `sm` resolves `command` against the original `PATH`. It then creates a wrapper with the same command name in a private directory placed first on the child shell's `PATH`. The wrapper executes the resolved command with configured arguments, configured environment, and any user-provided arguments.

For the Pi example, typing this inside the child shell:

```console
(sm) $ pi --model sonnet
```

executes the equivalent argv sequence:

```text
/path/to/pi --no-skills --skill /absolute/generation/skills --model sonnet
```

## Bundled Templates

A distribution may ship verified target templates. Bundled templates use the same schema as user configuration and contain no privileged core behavior. User configuration with the same target name overrides bundled data.

A template must only claim shell support when its command can receive an explicit skill directory through argv or environment. Commands restricted to a fixed global skill directory support persistent activation only.

## Safety

Configuration errors fail before a shell starts or a persistent target changes. In particular, `sm` rejects:

- unknown target names;
- relative persistent paths;
- shell templates without `{skills}`;
- commands that cannot be resolved;
- malformed argv or environment values.
