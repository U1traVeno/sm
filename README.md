# sm

`sm` keeps an agent's visible skills small by activating directory-based profiles on demand.

A Git repository under `~/.sm` stores every available skill. `sm enable` and `sm disable` expose only the required profiles in an agent's skill directory, using symlinks. Different profiles can be enabled for different targets, and `sm shell` can create an isolated skill set for commands that accept an explicit skill path.

`sm` is not a skill package manager and does not run Git. It manages skill visibility.

## Installation

The current implementation supports macOS and Linux and requires Rust 1.88 or newer to build:

```console
$ cargo install --path .
```

## Quick Start

Create or clone an `sm` home:

```text
~/.sm/
  profiles/
    common/
      code-review/
        SKILL.md
    research/
      web-search/
        SKILL.md
```

Configure a target in `~/.config/sm/config.toml`:

```toml
default_target = "pi"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

Enable profiles for the default target:

```console
$ sm enable common research
$ sm enabled
common
research
```

Later-enabled profiles win when two profiles contain the same skill directory name. Disabling the winner reveals the next enabled copy:

```console
$ sm disable research
```

Start an isolated child shell with an explicit profile set:

```console
$ sm shell pi --profile common --profile research
(sm) $ pi
```

Copy skills into a project when they should be committed with that project:

```console
$ sm export --profile common --skill research/web-search
```

This copies to `.skills/` by default. The project owns the copies; `sm` does not update or delete them later.

Import directories produced by an external installer, replacing selected existing skills explicitly:

```console
$ sm import --profile coding --from .agents/skills --replace
```

`sm` remains package-manager agnostic. See [Project Import and Export](docs/project-skills.md) for a composable `npx skills` workflow.

## Documentation

- [English documentation](docs/README.md)
- [简体中文文档](docs/zh-cn/README.md)

English topics:

- [Getting Started](docs/getting-started.md)
- [Profiles and Activation](docs/profiles.md)
- [Targets and Templates](docs/targets.md)
- [Isolated Shells](docs/shell.md)
- [Project Import and Export](docs/project-skills.md)
- [Command Reference](docs/command-reference.md)
- [Filesystem Layout](docs/filesystem-layout.md)

## Principles

- Profiles and skills are represented by directories.
- Git remains responsible for repository synchronization.
- Agent-specific behavior is data in target templates, not branches in the core.
- Successful mutating commands are silent.
- Diagnostics go to standard error.
- Existing files are never silently merged or overwritten; replacement must be explicit.
