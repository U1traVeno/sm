# Project Import and Export

Language: **English** | [简体中文](zh-cn/project-skills.md)

Project transfer copies skill directories. It is separate from persistent activation and isolated shell generations, which use symlinks.

A copy is a one-time ownership transfer:

- after export, the project owns the copy;
- after import, the destination profile owns the copy;
- `sm` does not track, update, synchronize, or later delete either copy.

## Export Profiles

Export one or more complete profiles:

```console
$ sm export --profile common --profile coding
```

The default destination is `.skills/` under the current working directory.

Export to an agent-recognized project directory explicitly:

```console
$ sm export --profile coding --to .agents/skills
```

`.skills/` is neutral vendored storage. `sm` does not assume that an agent discovers it automatically.

## Export Individual Skills

Use a profile-qualified reference:

```console
$ sm export --skill research/web-search
```

Combine complete profiles and individual skills:

```console
$ sm export \
  --profile common \
  --skill research/web-search \
  --to .skills
```

Selectors form a union and are resolved from left to right. A later selector wins when selected profiles contain the same skill directory name.

At least one `--profile` or `--skill` is required. Export never inherits a target's enabled profiles.

## Import into a Profile

Import every immediate skill directory from `.skills/` into a profile:

```console
$ sm import --profile project-tools
```

Import selected skills only:

```console
$ sm import \
  --profile project-tools \
  --skill deploy \
  --skill release-notes
```

Use a different source directory with `--from`:

```console
$ sm import --profile project-tools --from .agents/skills
```

The destination profile is created when it does not exist. Imported skills become ordinary directories under:

```text
$SM_HOME/profiles/<profile>/<skill>/
```

## Collision Rules

Before copying anything, `sm` validates every source and destination name. If any destination entry already exists, the entire operation fails without copying.

`sm` does not:

- merge directories;
- overwrite existing files;
- compare versions;
- update earlier copies;
- provide a force flag.

Use ordinary tools such as `diff -r`, `rm -r`, and `mv` to resolve a conflict, then rerun the command.

## Copy Semantics

`sm` recursively copies each selected skill directory and preserves executable permission bits and symlinks contained inside the skill. The top-level imported or exported skill becomes a real directory at the destination.

A successful import or export is silent. Use `--dry-run` to inspect the planned copies:

```console
$ sm export --profile common --dry-run
copy	/Users/me/.sm/profiles/common/code-review	/working/project/.skills/code-review
```
