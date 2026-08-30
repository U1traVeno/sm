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

Replace selected existing skill directories explicitly:

```console
$ sm import --profile project-tools --from .agents/skills --replace
```

The destination profile is created when it does not exist. Imported skills become ordinary directories under:

```text
$SM_HOME/profiles/<profile>/<skill>/
```

## Composing with Package Installers

`sm` does not invoke package installers. An installer can write into a temporary project directory, followed by `sm import --replace`. This zsh function composes `npx skills` with the default persistent target:

```zsh
sm-add() {
  if (( $# < 2 )); then
    print -u2 'usage: sm-add PROFILE SOURCE [SKILLS-ADD-OPTION...]'
    return 2
  fi

  local profile=$1 arg tmp
  shift
  for arg in "$@"; do
    case $arg in
      -g|--global|--global=*|-a|--agent|--agent=*|--all|--copy)
        print -u2 "sm-add: unsupported skills add option: $arg"
        return 2
        ;;
    esac
  done

  tmp=$(mktemp -d "${TMPDIR:-/tmp}/sm-add.XXXXXXXX") || return 1
  {
    (
      cd "$tmp" &&
        command npx --yes skills add "$@" --agent universal --yes
    ) || return
    command sm import \
      --profile "$profile" \
      --from "$tmp/.agents/skills" \
      --replace || return
    if ! command sm apply; then
      print -u2 "sm-add: imported into profile $profile, but sm apply failed"
      return 1
    fi
  } always {
    command rm -rf -- "$tmp"
  }
}
```

The first argument belongs to `sm`; the remaining arguments are passed to `skills add`. Scope, agent, all-agent, and copy options are rejected because they can bypass the temporary canonical directory. Set `SM_TARGET` when applying to a non-default target:

```console
$ SM_TARGET=pi sm-add development vercel-labs/agent-skills --skill web-design-guidelines
```

The function imports inventory but does not enable the profile or change precedence. Run `sm enable <profile>` explicitly the first time. The temporary `skills-lock.json` is discarded, so updates repeat the original `sm-add` command rather than using `npx skills update`. If import succeeds but apply fails, the imported profile remains; resolve the target problem and run `sm apply` again.

## Collision Rules

Before copying anything, `sm` validates every source and destination name. Without `--replace`, any existing destination entry fails the entire operation without copying.

With `--replace`, each selected existing destination must be a real directory. New content is fully staged before destination changes begin. Ordinary failures during installation roll back the selected names. Replacement does not merge directories, delete profile entries absent from the source, compare versions, or synchronize earlier copies.

Use ordinary tools such as `diff -r` and `rm -r` when you need comparison, merging, or deletion semantics.

## Copy Semantics

`sm` recursively copies each selected skill directory and preserves executable permission bits and symlinks contained inside the skill. The top-level imported or exported skill becomes a real directory at the destination.

A successful import or export is silent. Use `--dry-run` to inspect the planned copies:

```console
$ sm export --profile common --dry-run
copy	/Users/me/.sm/profiles/common/code-review	/working/project/.skills/code-review
```
