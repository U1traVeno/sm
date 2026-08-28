# sm

Language: **English** | [简体中文](zh-cn/)

`sm` keeps an agent's visible skills small by activating directory-based profiles on demand.

A Git repository under `~/.sm` stores every available skill. `sm enable` and `sm disable` expose only the required profiles in an agent's skill directory, using symlinks. Different profiles can be enabled for different targets, and `sm shell` can create an isolated skill set for commands that accept an explicit skill path.

`sm` is not a skill package manager and does not run Git. It manages skill visibility.

## Start Here

- [Getting Started](getting-started.md)
- [Profiles and Activation](profiles.md)
- [Targets and Templates](targets.md)
- [Isolated Shells](shell.md)
- [Project Import and Export](project-skills.md)

## Reference

- [Command Reference](command-reference.md)
- [Filesystem Layout](filesystem-layout.md)
