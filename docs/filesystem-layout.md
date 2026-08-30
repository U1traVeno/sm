# Filesystem Layout

Language: **English** | [简体中文](zh-cn/filesystem-layout.md)

`sm` separates synchronized skill content, user configuration, machine-local state, and disposable cache data.

## Skill Repository

```text
${SM_HOME:-~/.sm}/
  profiles/
    <profile>/
      <skill>/
        SKILL.md
        ...
```

This is the only directory intended for Git synchronization. `sm` does not require it to be a Git repository and never invokes Git.

## Configuration

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

Configuration contains the default target and generic target/template data. See [Targets and Templates](targets.md).

## Local State

```text
${XDG_STATE_HOME:-~/.local/state}/sm/
  targets/
    <target>/
      enabled/
        <sequence>-<profile>/
      lock/
  profiles/
    <profile>/
      lock/
  leases/
    <generation-id>/
      ...
```

Enabled markers are empty directories. Per-target locks serialize persistent updates, and per-profile locks serialize imports into the same profile. Lock directories exist only while an operation is running. Lease entries prevent `sm gc` from deleting generations used by live child shells.

State is machine-local and must not be committed to the skill repository.

## Cache

```text
${XDG_CACHE_HOME:-~/.cache}/sm/
  generations/
    <generation-id>/
      skills/
        <skill> -> $SM_HOME/profiles/<profile>/<skill>
      bin/
        <configured-command-wrapper>
      shell/
        bash/
          bashrc
          bashenv
        zsh/
          .zshenv
          .zprofile
          .zshrc
          .zlogin
          .zlogout
```

Generation IDs are opaque implementation details. A generation's link membership and targets do not change after creation. Skill contents remain mutable through the source symlinks. The generated Bash and zsh files source the user's real startup files and then restore command-wrapper precedence; Fish uses an equivalent `--init-command`. These files are cache data, not user configuration.

The complete cache can be reconstructed. Use `sm gc` for normal cleanup; do not remove a generation used by a live child shell.

## Persistent Agent Targets

Persistent target directories are outside `sm`'s own state and are declared in configuration. For example:

```text
~/.pi/agent/skills/
  code-review -> ~/.sm/profiles/coding/code-review
  web-search  -> ~/.sm/profiles/research/web-search
  manually-installed-skill/
```

`sm` preserves unmanaged entries. It only removes symlinks whose stored target identifies a skill under `$SM_HOME/profiles`.

## Project Copies

The default import/export directory is relative to the current project:

```text
<project>/.skills/
  <skill>/
```

These are ordinary copied directories, not `sm` state. The project owns exported copies, and `sm` does not later synchronize or remove them.
