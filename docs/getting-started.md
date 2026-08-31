# Getting Started

Language: **English** | [简体中文](zh-cn/getting-started.md)

## 1. Install

```console
cargo install sm-skill-manager
```

The crate installs `sm` and requires Rust 1.88 or newer.

## 2. Create a Profile

```console
sm profiles new common
```

This creates:

```text
~/.sm/profiles/common/.smtag
```

with `true`. A profile directory created manually is also valid; sm initializes a missing `.smtag` to `true` when it next reads the inventory.

Add skill directories directly or import them:

```console
sm import --profile common --from .skills
```

## 3. Configure a Target

Register a persistent projection directory:

```console
sm adopt agents ~/.agents/skills
```

Or write `~/.config/sm/config.toml`:

```toml
[targets.agents]
skills_dir = "~/.agents/skills"
```

A target never owns activation state. Every target receives the same global set.

## 4. Apply

```console
sm apply
```

This creates symlinks for all skills in profiles whose `.smtag` is `true`.

If an installer has written real directories into the target, inspect first:

```console
sm apply --force --dry-run
```

Then reconcile:

```console
sm apply --force
```

Force removes all non-hidden real directories from selected targets. Ordinary files, hidden entries, and unrelated external symlinks are preserved.

## 5. Enable and Disable

```console
sm disable common
sm apply

sm enable common
sm apply
```

Enable and disable only update `.smtag`; apply is always explicit.

## 6. Add Installer Skills

After an external installer writes to a target, import only the new skills you want:

```console
sm profiles new matt
sm import --profile matt \
  --from ~/.agents/skills \
  --skill code-review \
  --skill tdd
sm apply --force
```

Import leaves source directories unchanged. Force apply removes installer directories and restores the desired symlinks.

## 7. Update Managed Skills

Download broadly, then update only names already present in sm:

```console
npx skills@latest add mattpocock/skills
sm update --from ~/.agents/skills --profile matt
sm apply --force
```

Unknown downloads are never added by update.

## 8. Configure Isolated Shells

Add an adapter:

```toml
[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

Start a temporary profile scope:

```console
sm shell --profile matt
(sm) $ pi
```

With no explicit profiles, shell generations inherit the global set. With no target filter, all configured adapters are wrapped.

## 9. Inspect

```console
sm profiles
sm skills
sm enabled
sm targets
sm status
```

Use `--dry-run` on mutating commands to validate planned operations without changing files.
