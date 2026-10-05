# sm

`sm` manages directory-based agent skill profiles, keeps one global activation set, and projects that set into configured agent skill directories with symlinks.

The complete inventory lives under `~/.sm/profiles`. Each profile owns a `.smtag` containing `true` or `false`. Every configured target receives the same globally enabled skill set. Temporary variation belongs to `sm shell`, which gives configured agent commands a session-specific generated skill directory without changing global targets.

`sm` is package-manager agnostic. It does not invoke Git, run installers, or parse `SKILL.md`.

## Installation

The current implementation supports macOS and Linux and requires Rust 1.88 or newer:

```console
cargo install sm-skill-manager
```

The crate installs the `sm` executable. To install a checkout:

```console
cargo install --path .
```

## Profile Layout

```text
~/.sm/
  profiles/
    common/
      .smtag              # true
      code-review/
        SKILL.md
    archive/
      .smtag              # false
      old-skill/
        SKILL.md
```

A missing `.smtag` is initialized to `true` when sm next reads the inventory. Profile and skill identity come from immediate directory names.

Create and activate profiles:

```console
sm profiles new matt
sm profiles new archive --disabled
sm enable matt
sm disable archive
```

Activation commands change inventory state only. Run `sm apply` separately to update targets.

## Targets

Configure projection directories and optional shell adapters in `~/.config/sm/config.toml`:

```toml
[targets.agents]
skills_dir = "~/.agents/skills"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

Target paths must be distinct and must not overlap the profile inventory. All persistent targets receive the same global set:

```console
sm apply
sm apply --target agents
sm apply --force
```

A skill copy (a real directory containing `SKILL.md`) in a target blocks normal apply. `--force` removes non-hidden skill copies before creating the desired links, which is useful after an external installer writes directly into a target. Real directories without `SKILL.md`, such as an agent's own `synced/` cache, belong to another owner and are always preserved.

Register another projection directory without hand-editing TOML:

```console
sm adopt codex ~/.codex/skills
```

## External Installers

Import selected new skills into a profile:

```console
npx skills@latest add mattpocock/skills
sm import --profile matt --from ~/.agents/skills --create \
  --skill code-review --skill tdd
sm apply --force
```

Update only skills already present anywhere in the inventory:

```console
npx skills@latest add mattpocock/skills
sm update --from ~/.agents/skills
sm apply --force
```

`update` never creates profiles or skills. Unknown source directories remain untouched. Duplicate inventory names require `--profile PROFILE` or `--all`.

## Isolated Shells

```console
sm shell --profile matt
(sm) $ pi
```

With no `--target`, every configured shell adapter is wrapped in the child zsh, Bash, or Fish session. With no `--profile`, the generation inherits globally enabled profiles. Any explicit `--profile` replaces that inherited set.

## Project Copies

```console
sm export --profile common --to .agents/skills
sm import --profile project-tools --from .skills --create
```

Import and export copy directories. Their source copies remain independently owned.

## Documentation

- [Getting Started](docs/getting-started.md)
- [Profiles and Global Activation](docs/profiles.md)
- [Targets and Shell Adapters](docs/targets.md)
- [Isolated Shells](docs/shell.md)
- [Import, Update, and Export](docs/project-skills.md)
- [Command Reference](docs/command-reference.md)
- [Filesystem Layout](docs/filesystem-layout.md)
- [简体中文文档](docs/zh-cn/README.md)

## Principles

- Profile directories are the source of inventory truth.
- Global activation belongs to each profile, not to a target.
- Inventory commands and persistent projection are separate phases.
- `apply` is the only command that changes persistent target contents.
- Temporary differences use isolated shell generations.
- Git and package installers remain separate tools.
