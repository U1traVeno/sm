# Command Reference

Language: **English** | [简体中文](zh-cn/command-reference.md)

## Common Conventions

- `-t` is short for `--target`.
- Paths beginning with `~` are expanded where configuration allows paths.
- Profile and target names are single directory components.
- A skill reference has the form `<profile>/<skill>`.
- Mutating commands are silent on success unless `--dry-run` is used.
- Primary output goes to standard output; diagnostics go to standard error.

## `sm profiles`

List profile names, one per line in bytewise lexical order.

```text
sm profiles
```

## `sm skills`

List profile-qualified skill names.

```text
sm skills [PROFILE...]
```

With no profile arguments, list skills from every profile. Output is one `<profile>/<skill>` per line in bytewise lexical order.

## `sm targets`

List configured target names, one per line.

```text
sm targets
```

## `sm enabled`

Print enabled profiles for one persistent target from lowest to highest precedence.

```text
sm enabled [-t TARGET]
```

## `sm status`

Print the currently materialized winning skills for one persistent target.

```text
sm status [-t TARGET]
```

Each line is tab-separated:

```text
<skill-name>\t<absolute-source>\t<absolute-destination>
```

An empty target produces no output and exits successfully.

## `sm enable`

Enable profiles for one persistent target.

```text
sm enable PROFILE... [-t TARGET] [--dry-run]
```

Profiles are processed left to right. Each named profile moves to highest precedence. The resulting union is preflighted and materialized as managed symlinks.

## `sm disable`

Disable profiles for one persistent target.

```text
sm disable PROFILE... [-t TARGET] [--dry-run]
```

Profiles are processed left to right. Disabling an already-disabled profile is successful and still reconciles the target.

## `sm apply`

Reconcile one persistent target without changing enabled profiles or precedence.

```text
sm apply [-t TARGET] [--dry-run]
```

Use this after changing or updating profile directories, or to repair an interrupted target update.

## `sm shell`

Start an isolated child shell using a target's generic shell template.

```text
sm shell TARGET \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [-- SHELL-ARGUMENT...]
```

Selection behavior:

- with any `--profile`, use only explicitly selected profiles plus `--skill` additions;
- without `--profile`, inherit the target's persistent enabled profiles and add `--skill` selections.

Arguments after `--` are passed to the child shell, not the wrapped agent command.

## `sm export`

Copy profiles or individual skills into a project-owned directory.

```text
sm export \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [--to DIRECTORY] \
  [--dry-run]
```

At least one selector is required. The default destination is `.skills/`. Existing destination names fail the entire operation.

## `sm import`

Copy project-owned skills into one profile.

```text
sm import \
  --profile PROFILE \
  [--skill SKILL]... \
  [--from DIRECTORY] \
  [--replace] \
  [--dry-run]
```

The default source is `.skills/`. With no `--skill`, import every immediate skill directory. The destination profile is created if absent. Existing destination names fail the entire operation unless `--replace` is explicit. Replacement affects selected names only, requires existing destinations to be real directories, and preserves every other profile entry.

## `sm gc`

Remove unleased shell generations.

```text
sm gc [--dry-run]
```

Live generations are never removed.

## Dry-Run Output

`--dry-run` performs validation and prints the planned operations without mutation. Output is tab-separated, one operation per line. Operation names are lowercase:

```text
enable\t<TARGET>\t<PROFILE>
disable\t<TARGET>\t<PROFILE>
link\t<SOURCE>\t<DESTINATION>
unlink\t<DESTINATION>
copy\t<SOURCE>\t<DESTINATION>
replace\t<SOURCE>\t<DESTINATION>
remove\t<PATH>
```

Paths are absolute.

## Exit Status

- `0`: success, including idempotent no-op operations.
- `1`: operational failure such as a missing profile, collision, invalid target, or copy error.
- `2`: command-line usage error.

Expected failures make no target or destination changes. An operating-system interruption may leave a partially reconciled persistent target or hidden import staging/backup directory. Rerun `sm apply` for a target update; inspect the affected profile before removing interrupted import data.
