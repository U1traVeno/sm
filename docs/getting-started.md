# Getting Started

Language: **English** | [简体中文](zh-cn/getting-started.md)

## Requirements

The current implementation supports macOS and Linux. Building from source requires Rust 1.88 or newer.

`sm` needs a filesystem that can create symbolic links for persistent activation and shell generations. Project import and export use copies instead.

Git is optional at runtime. Use ordinary Git commands when `~/.sm` is a Git checkout.

## Installation

Install from crates.io:

```console
$ cargo install sm-skill-manager
```

The crate is named `sm-skill-manager`; the installed executable is `sm`. To install from a project checkout instead:

```console
$ cargo install --path .
```

## Create the Skill Repository

`SM_HOME` defaults to `~/.sm`. A profile is an immediate child of `profiles/`, and a skill is an immediate directory inside a profile:

```text
~/.sm/
  profiles/
    common/
      shell-tools/
        SKILL.md
    coding/
      code-review/
        SKILL.md
      repository-search/
        SKILL.md
```

`sm` identifies profiles and skills by directory name. It does not parse or validate `SKILL.md`.

To synchronize the inventory between machines, make `~/.sm` a normal Git checkout:

```console
$ git clone git@example.com:you/skills.git ~/.sm
$ git -C ~/.sm pull --ff-only
```

`sm` never runs these commands itself.

## Configure a Target

Create `~/.config/sm/config.toml`:

```toml
default_target = "pi"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

The persistent target and shell template are independent capabilities:

- `skills_dir` allows `enable` and `disable`.
- `shell` allows isolated `sm shell` sessions.
- A target may define either capability or both.

See [Targets and Templates](targets.md) for the complete format.

## Activate Profiles

```console
$ sm enable common coding
$ sm enabled
common
coding
```

The order shown by `sm enabled` is precedence order from lowest to highest. Enabling an already-enabled profile moves it to the end:

```console
$ sm enable common
$ sm enabled
coding
common
```

If both profiles contain `code-review`, the copy under `common` is now visible.

Disable a profile without affecting the others:

```console
$ sm disable common
```

Use `--target` when no default target is configured or when activating a different target:

```console
$ sm enable research --target claude
```

## Inspect State

List available data with line-oriented output:

```console
$ sm profiles
coding
common
research

$ sm skills coding
coding/code-review
coding/repository-search

$ sm targets
claude
pi
```

Inspect the materialized winners for a target:

```console
$ sm status --target pi
code-review	/Users/me/.sm/profiles/coding/code-review	/Users/me/.pi/agent/skills/code-review
```

Paths in actual output are absolute. Fields are separated by tabs.

## Next Steps

- Read [Profiles and Activation](profiles.md) for precedence and collision behavior.
- Read [Isolated Shells](shell.md) before running concurrent agents with different skill sets.
- Read [Project Import and Export](project-skills.md) to vendor skills into a repository.
