# Profiles and Global Activation

Language: **English** | [简体中文](zh-cn/profiles.md)

## Layout

A profile is an immediate directory under `$SM_HOME/profiles`:

```text
$SM_HOME/profiles/<profile>/
  .smtag
  <skill>/
    SKILL.md
```

A skill is an immediate non-hidden directory. sm treats its contents as opaque and does not parse `SKILL.md`. Profile and skill entries must be real directories, not symlinks.

Deleting a profile directory deletes that profile and its activation state. Creating a profile directory creates a profile immediately.

## `.smtag`

`.smtag` contains exactly one Boolean value, with an optional trailing newline:

```text
true
```

or:

```text
false
```

`true` means the profile belongs to the globally active set. Every persistent target receives this same set.

When sm discovers a profile without `.smtag`, it creates the file with `true` and reports the initialization on standard error. Dry-run commands report the planned tag without writing it. Any other tag value is an error.

Create profiles explicitly:

```console
sm profiles new matt
sm profiles new archive --disabled
```

Change global activation:

```console
sm enable matt
sm disable archive
```

These commands only update `.smtag`. They do not modify targets. Run `sm apply` when the inventory is ready.

## Duplicate Skill Names

Different profiles may store the same skill name, but two globally enabled profiles may not expose that name simultaneously:

```text
profiles/old/code-review/
profiles/matt/code-review/
```

This inventory is valid if at most one owner is enabled. `sm enable`, an enabled import, and `sm apply` reject an active duplicate before changing anything.

An isolated shell may explicitly select profiles with duplicate skill names. There, selector order is temporary and later selectors win; no precedence is persisted.

## Manual Editing

The filesystem is authoritative. You may:

- create or delete profile directories;
- move skill directories between profiles;
- edit `.smtag` directly;
- edit skill contents in place.

Run `sm apply` after membership or activation changes. Run `sm update --from DIRECTORY` when an external installer has produced newer copies of already managed skills.

## Legacy Activation State

Older releases stored per-target enabled marker directories under the XDG state directory. The first non-dry-run inventory command removes those obsolete markers. They are never consulted by the global activation model.
