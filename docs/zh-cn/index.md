# sm

语言：[English](../) | **简体中文**

`sm` 通过按需激活目录式 profiles，控制 Agent 能看到的 skills 数量。

`~/.sm` 下的 Git 仓库存放全部 skills。`sm enable` 和 `sm disable` 使用符号链接，只把所需 profiles 暴露到 Agent 的 skill 目录。不同 targets 可以启用不同 profiles；对于支持显式 skill 路径的命令，`sm shell` 还能创建隔离的 skill 集合。

`sm` 不是 skill 包管理器，也不会运行 Git。它只管理 skill 的可见性。

## 从这里开始

- [快速开始](getting-started.md)
- [Profiles 与激活](profiles.md)
- [Targets 与模板](targets.md)
- [隔离 Shell](shell.md)
- [项目 Skill 的导入与导出](project-skills.md)

## 参考

- [命令参考](command-reference.md)
- [文件系统布局](filesystem-layout.md)
