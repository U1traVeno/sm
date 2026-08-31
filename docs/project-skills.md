# Import, Update, and Export

Language: **English** | [简体中文](zh-cn/project-skills.md)

sm deliberately separates inventory changes from target projection:

```text
import/update/enable/disable -> profile inventory
apply                        -> persistent targets
```

This makes external installers composable without giving them ownership of `$SM_HOME`.

## Import New Skills

Import copies source directories into one profile:

```console
sm import --profile project-tools --from .skills --create
```

Rules:

- the destination profile must exist unless `--create` is explicit;
- a created profile receives `.smtag` with `true`;
- import may add skills but never deletes source directories;
- existing destination names fail unless `--replace` is explicit;
- replacement affects selected names only;
- ordinary files and other non-directory source entries are ignored;
- sm-managed links in the source are skipped;
- other source symlinks are rejected;
- the complete batch is staged before replacement;
- import never runs apply.

Select a subset:

```console
sm import --profile matt \
  --from ~/.agents/skills \
  --create \
  --skill code-review \
  --skill tdd
```

If adding a skill would create a duplicate name across globally enabled profiles, import fails before changing inventory. Disable one owner first or import into a disabled profile.

## Update Existing Skills

`update` searches all profiles by immediate skill directory name and replaces only existing matches:

```console
sm update --from ~/.agents/skills
```

Unknown source directories remain in place and are not added to sm.

When one name exists in several profiles, activation does not choose an owner. Resolve the ambiguity explicitly:

```console
sm update --from ~/.agents/skills --profile matt
sm update --from ~/.agents/skills --all
```

`--profile` limits updates to existing matches in one existing profile. `--all` updates every existing copy. Neither option allows update to create a profile or skill.

Select source names when needed:

```console
sm update --from ~/.agents/skills --skill code-review --profile matt
```

An explicitly selected source name must exist. The update batch is fully staged before any inventory destination changes.

## npx Workflow

Install broadly into a configured target:

```console
npx skills@latest add mattpocock/skills
```

For new skills, choose what to manage:

```console
sm profiles new matt
sm import --profile matt \
  --from ~/.agents/skills \
  --skill code-review \
  --skill tdd
sm apply --force
```

Real source directories remain until force apply removes them and recreates links for globally active skills. Unwanted installer directories are also removed from selected targets by `--force`.

For later bulk updates:

```console
npx skills@latest add mattpocock/skills
sm update --from ~/.agents/skills --profile matt
sm apply --force
```

This updates known inventory entries, ignores newly offered skills, and then restores target links.

## Export Project Copies

Export copies selected inventory skills to a project-owned directory:

```console
sm export --profile common
sm export --skill research/web-search --to .agents/skills
```

At least one selector is required. Later selectors win duplicate output names. Existing destination entries are never overwritten. The project owns exported copies and sm does not update or remove them later.

## Dry Run

```console
sm import --profile matt --from ~/.agents/skills --create --dry-run
sm update --from ~/.agents/skills --dry-run
sm apply --force --dry-run
```

Dry runs validate and print tab-separated operations without creating tags, inventory entries, links, or directories.
