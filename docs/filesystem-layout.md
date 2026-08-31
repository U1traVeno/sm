# Filesystem Layout

Language: **English** | [简体中文](zh-cn/filesystem-layout.md)

sm separates inventory, configuration, process coordination, generated cache data, and persistent projections.

## Inventory

```text
${SM_HOME:-~/.sm}/
  profiles/
    <profile>/
      .smtag
      <skill>/
        SKILL.md
        ...
```

Profile directories and their `.smtag` files are the source of inventory and activation truth. This tree may be managed with Git; sm never invokes Git.

A missing `.smtag` is initialized to `true`. Hidden profile entries are metadata and are not skills. Skill contents are opaque.

## Configuration

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

Configuration contains named targets. Each target may have a persistent `skills_dir`, a shell adapter, or both. `sm adopt` edits only the selected target's `skills_dir` while preserving existing TOML formatting and comments.

## Local State

```text
${XDG_STATE_HOME:-~/.local/state}/sm/
  inventory/
    lock/
  targets/
    <target>/
      lock/
  leases/
    <generation-id>/
      <pid-nonce>/
```

Lock directories exist only while an operation is running. Inventory locking serializes tag/import/update changes. Target locks serialize apply operations. Lease entries prevent garbage collection of generations used by live shells or wrapped commands.

There is no persistent activation state under XDG state. Legacy `targets/<target>/enabled/` directories are removed when the inventory is initialized.

## Cache

```text
${XDG_CACHE_HOME:-~/.cache}/sm/
  generations/
    <generation-id>/
      skills/
        <skill> -> $SM_HOME/profiles/<profile>/<skill>
      bin/
        <agent-wrapper>
      shell/
        bash/
        zsh/
```

Generation IDs represent the selected skill paths, shell adapters, wrapper executable paths, and sm executable. Membership is immutable after generation creation, while linked skill contents remain mutable.

Use `sm gc` to remove complete generations without live leases.

## Persistent Targets

Configured target directories exist outside sm state:

```text
~/.agents/skills/
  code-review -> ~/.sm/profiles/matt/code-review
  web-search  -> ~/.sm/profiles/research/web-search
```

Every target receives the same globally active set. Normal apply manages links under `$SM_HOME/profiles` and rejects real directories. Force apply removes all non-hidden real directories, then installs desired links. Ordinary files, hidden entries, and unrelated external links remain unmanaged.

## Project and Installer Copies

Import, update, and export operate on ordinary copied directories. Import and update leave their sources in place; force apply may later remove real directories when the source is itself a configured target.
