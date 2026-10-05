# Command Reference

Language: **English** | [简体中文](zh-cn/command-reference.md)

## Conventions

- Profile, skill, and target names are one non-hidden directory component.
- A skill reference is `<profile>/<skill>`.
- Global activation comes from `<profile>/.smtag`.
- Successful mutations are silent except one-time migration diagnostics.
- `--dry-run` validates and prints operations without mutation.
- `-t` is short for `--target`; `-f` is short for `--force`.

## `sm profiles`

List profile names:

```text
sm profiles
```

Create profiles:

```text
sm profiles new PROFILE... [--disabled] [--dry-run]
```

New profiles are enabled unless `--disabled` is supplied.

## `sm skills`

```text
sm skills [PROFILE...]
```

Print `<profile>/<skill>` lines. With no arguments, inspect every profile.

## `sm targets`

```text
sm targets
```

List configured target names.

## `sm enabled`

```text
sm enabled
```

List globally enabled profiles in lexical order.

## `sm status`

```text
sm status [-t TARGET]
```

Without a target, each tab-separated line is:

```text
<target>\t<skill>\t<source>\t<destination>
```

With a target, the first column is omitted.

## `sm enable` and `sm disable`

```text
sm enable PROFILE... [--dry-run]
sm disable PROFILE... [--dry-run]
```

Write `true` or `false` to profile tags. These commands never modify targets. Enabling a profile that conflicts with another enabled owner fails before writing tags.

## `sm apply`

```text
sm apply [-t TARGET] [-f|--force] [--dry-run]
```

Project globally enabled skills to all persistent targets, or one selected target. All selected targets are preflighted before mutation.

Without force, any non-hidden skill copy (a real directory containing `SKILL.md`) blocks the entire operation. With force, those directories are removed. Real directories without `SKILL.md` belong to another owner and are always preserved. Managed links are repaired or removed as needed. Files, hidden entries, and unrelated external symlinks are preserved unless they block a desired name.

## `sm shell`

```text
sm shell \
  [--target TARGET]... \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [-- SHELL_ARGUMENT...]
```

Start a child shell with one stable generation. Without target filters, wrap every configured shell adapter. Without profiles, inherit globally enabled profiles. Any profile option replaces inheritance; skill options add individual entries.

## `sm import`

```text
sm import \
  --profile PROFILE \
  [--skill SKILL]... \
  [--from DIRECTORY] \
  [--create] [--replace] [--dry-run]
```

Copy source directories into one profile. The default source is `.skills`. Import may add skills and explicitly create a profile, but leaves all source directories in place. Ordinary non-directory entries are ignored, managed source links are skipped, and other source symlinks are rejected.

## `sm update`

```text
sm update \
  --from DIRECTORY \
  [--skill SKILL]... \
  [--profile PROFILE | --all] \
  [--dry-run]
```

Replace only existing same-named inventory skills. Unknown source directories and ordinary non-directory entries are skipped and never imported. Duplicate inventory names require one profile or all existing owners. Source directories remain in place.

## `sm export`

```text
sm export \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [--to DIRECTORY] [--dry-run]
```

Copy selected inventory skills to a project-owned directory. At least one selector is required. Existing destination names are never overwritten.

## `sm adopt`

```text
sm adopt TARGET DIRECTORY [--dry-run]
```

Register a normalized absolute `skills_dir` for a target while preserving existing TOML formatting and shell configuration. Adopt does not create the target directory or run apply.

## `sm gc`

```text
sm gc [--dry-run]
```

Remove generated isolated-shell directories without live leases.

## Dry-Run Operations

Output is tab-separated and may include:

```text
tag\ttrue\t<path>
create-profile\t<enabled>\t<path>
enable\t<profile>
disable\t<profile>
copy\t<source>\t<destination>
replace\t<source>\t<destination>
update\t<source>\t<destination>
skip\t<source>
adopt\t<target>\t<directory>
link\t<source>\t<destination>
unlink\t<destination>
remove\t<path>
```

## Exit Status

- `0`: success, including idempotent no-op operations;
- `1`: operational or validation failure;
- `2`: command-line usage error.

Expected validation failures occur before inventory or selected targets change. An OS interruption during apply may leave some planned links reconciled; rerun `sm apply`.
