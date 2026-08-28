# Profiles and Activation

Language: **English** | [简体中文](zh-cn/profiles.md)

## Profile Layout

A profile is a directory under `$SM_HOME/profiles`:

```text
$SM_HOME/profiles/<profile>/<skill>/
```

Only immediate, non-hidden directories are skills. A profile must not use files or symlinks as membership entries. Content below each skill directory is opaque to `sm`.

For example:

```text
~/.sm/profiles/
  common/
    web-search/
    code-review/
  backend/
    database-migrations/
    code-review/
```

A skill that conceptually belongs to several groups should live in a separate profile such as `common`. `sm` does not maintain a central skill catalog or profile membership links.

## Enabling Profiles

Profiles are enabled independently for each target:

```console
$ sm enable common backend --target pi
```

Arguments are processed from left to right. A profile enabled later has higher precedence. Enabling an already-enabled profile raises it to the highest precedence without duplicating it.

`sm enable` performs these steps:

1. Validate every named profile and all immediate skill entries.
2. Resolve the union of enabled profiles.
3. Check the target for unmanaged collisions.
4. Record the new precedence using filesystem marker directories.
5. Materialize the winning skills as symlinks.

Expected validation failures happen before the target is changed.

## Disabling Profiles

```console
$ sm disable backend --target pi
```

Disabling removes the profile from that target's enabled set and rematerializes the remaining union. The command is idempotent: disabling a profile that is already disabled still reconciles the target and succeeds silently.

## Duplicate Skill Names

The immediate directory name is the skill identity. `sm` does not compare `SKILL.md` frontmatter or file contents.

Given this order:

```text
common
backend
```

and these directories:

```text
profiles/common/code-review/
profiles/backend/code-review/
```

`backend/code-review` wins because `backend` was enabled later. If `backend` is disabled, `common/code-review` becomes visible again.

## Managed and Unmanaged Entries

`sm` manages only symlinks whose stored target points to a skill under `$SM_HOME/profiles/<profile>/<skill>`.

It does not remove or replace:

- regular files;
- regular directories;
- symlinks outside `$SM_HOME/profiles`;
- unrelated broken symlinks.

If an unmanaged entry has the same name as a selected skill, the operation fails before changing the target. Resolve the collision explicitly and rerun the command.

## Local State

Enabled state uses directories rather than a manifest:

```text
~/.local/state/sm/targets/pi/enabled/
  00000000000000000001-common/
  00000000000000000002-backend/
```

The numeric prefix stores precedence. Marker directories are empty. Re-enabling a profile replaces its old marker with a new highest sequence.

This state is machine-local and must not be committed to the skill repository.

## Synchronizing the Repository

Run Git separately:

```console
$ git -C ~/.sm pull --ff-only
$ sm enable common --target pi
```

Re-enabling reconciles the target and raises `common` to highest precedence. To reconcile without changing precedence, run:

```console
$ sm apply --target pi
```

`sm apply` uses the existing marker order and repairs managed links without changing enabled state.
