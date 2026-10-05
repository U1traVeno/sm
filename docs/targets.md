# Targets and Shell Adapters

Language: **English** | [简体中文](zh-cn/targets.md)

A target describes either or both of these capabilities:

1. a persistent skill directory where `sm apply` projects the globally active skill set;
2. a shell adapter that can start an agent command with an explicit generated skill directory.

Targets never select profiles and contain no activation state.

## Configuration

The default path is:

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

Example:

```toml
[targets.agents]
skills_dir = "~/.agents/skills"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

`skills_dir` must resolve to an absolute path. The directory may be absent; `sm apply` creates it. Two targets may not use the same path, and a target may not overlap `$SM_HOME/profiles` in either direction.

The old `default_target` key is accepted for compatibility but no longer controls activation or default apply behavior.

## Registering Targets

```console
sm adopt agents ~/.agents/skills
```

`adopt` adds `skills_dir` to the named target while preserving TOML formatting, comments, and existing shell configuration. It is idempotent for the same name and normalized path. It rejects name/path remapping and duplicate registration of one path under two names.

`adopt` does not create the directory, infer an agent type, add a shell adapter, or run apply.

## Persistent Projection

```console
sm apply
```

With no target option, every target with `skills_dir` is preflighted before any target is changed. All receive the same desired links.

```console
sm apply --target agents
```

This narrows one repair operation but does not create target-specific activation state.

Normal apply:

- repairs missing or stale sm-managed links;
- removes obsolete sm-managed links;
- preserves hidden entries, regular files, external symlinks, and real directories without `SKILL.md`;
- fails if any non-hidden skill copy (a real directory containing `SKILL.md`) is present;
- fails if a preserved entry blocks a desired skill name.

`sm apply --force` removes every non-hidden skill copy in selected targets before linking. It still preserves ordinary files, hidden entries, unrelated external symlinks, and real directories without `SKILL.md`, which belong to another owner.

## Shell Adapters

A shell adapter is data, not shell source:

```toml
[targets.example.shell]
command = "/absolute/path/to/example-agent"
args = ["--skills-dir", "{skills}"]
env = { EXAMPLE_MODE = "isolated" }
```

`{skills}` must appear in `args` or `env`. sm resolves the command before starting the child shell and creates a wrapper with the same basename. The wrapper injects the generated skills path without using `eval`, word splitting, or command substitution.

An agent that only reads a fixed global directory cannot be isolated. Leave it without a shell adapter; inside `sm shell` it continues to see the global projection.
