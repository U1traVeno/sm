# Isolated Shells

Language: **English** | [简体中文](zh-cn/shell.md)

`sm shell` starts a child interactive shell in which one configured command uses a stable, session-specific skill set.

It does not modify the parent shell and does not replace fixed global skill directories.

## Basic Use

```console
$ sm shell pi --profile common --profile research
(sm) $ pi
```

The child shell inherits the current working directory and ordinary environment. `sm` prepends a private wrapper directory to `PATH`. Only the configured command is wrapped; other commands behave normally.

Exit the child shell to end the scope:

```console
(sm) $ exit
$
```

Agents already started from the child shell continue to use their generation until they exit.

## Profile Selection

When at least one `--profile` is present, the profile set is explicit and does not inherit persistent enabled profiles:

```console
$ sm shell pi --profile common --profile coding
```

When no `--profile` is present, `sm shell` inherits the target's persistent enabled profiles:

```console
$ sm shell pi
```

Individual skills may be added with a profile-qualified reference:

```console
$ sm shell pi --skill research/web-search
```

With no `--profile`, this adds `research/web-search` to the inherited enabled set. With one or more `--profile`, it adds the skill to the explicit set.

Selectors are resolved from left to right. Later selectors win duplicate directory names.

## Stable Generations

A generation is a symlink mapping stored under the cache directory:

```text
~/.cache/sm/generations/<generation-id>/
  skills/
    code-review -> ~/.sm/profiles/coding/code-review
    web-search  -> ~/.sm/profiles/research/web-search
```

The generation ID represents the resolved source paths and precedence. Once created, its membership and link targets are never changed in place. A different resolved set creates or reuses a different generation.

This prevents two child shells from overwriting each other's visible skill membership. It also keeps a generation path valid when persistent profiles are later enabled or disabled.

The skill contents are not immutable. Because generation entries are symlinks, editing or removing a source skill changes what an existing generation sees. Do not rewrite the skill repository while a running agent requires a reproducible skill body.

## Agent Independence

`sm shell` is not Pi-specific. It uses the generic command template documented in [Targets and Templates](targets.md).

A command supports isolated shells only if it can receive a skill directory through argv or environment. If it only scans a fixed global directory, `sm shell` fails for that target. Use persistent `enable` and `disable` instead.

## Garbage Collection

While the child shell is alive, `sm` holds a lease on its generation. `sm gc` removes only generations without an active lease:

```console
$ sm gc --dry-run
remove	/Users/me/.cache/sm/generations/old-id
$ sm gc
```

Garbage collection removes complete generation directories. It never edits a generation in place.

## Failure Behavior

`sm shell` fails before starting the child shell when:

- the target is unknown;
- the target has no shell template;
- a selected profile or skill does not exist;
- duplicate resolution produces an unmanaged conflict inside the generation path;
- the configured command cannot be resolved;
- generation creation is incomplete.
