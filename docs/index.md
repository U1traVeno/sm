# sm

Language: **English** | [简体中文](zh-cn/)

`sm` stores directory-based skill profiles, keeps one global activation set in profile-local `.smtag` files, and projects that set to configured agent directories with symlinks.

Inventory operations are separate from persistent projection: use import or update to change profiles, then use apply to reconcile targets. `sm shell` provides temporary per-session skill sets to every configured shell adapter without changing global targets.

sm is package-manager agnostic and never invokes Git.

## Start Here

- [Getting Started](getting-started.md)
- [Profiles and Global Activation](profiles.md)
- [Targets and Shell Adapters](targets.md)
- [Isolated Shells](shell.md)
- [Import, Update, and Export](project-skills.md)

## Reference

- [Command Reference](command-reference.md)
- [Filesystem Layout](filesystem-layout.md)
