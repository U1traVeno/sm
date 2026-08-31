# sm

语言：[English](../) | **简体中文**

`sm` 保存目录式 skill profiles，通过 profile 内的 `.smtag` 维护一套全局启用集合，并使用软链接把该集合投影到 configured agent directories。

Inventory 操作与持久投影明确分离：使用 import 或 update 修改 profiles，再通过 apply 协调 targets。`sm shell` 为所有 configured shell adapters 提供会话专属 skill 集合，不修改全局 targets。

sm 与 package manager 解耦，也永远不会调用 Git。

## 从这里开始

- [快速开始](getting-started.md)
- [Profiles 与全局启用](profiles.md)
- [Targets 与 Shell Adapters](targets.md)
- [隔离 Shell](shell.md)
- [Import、Update 与 Export](project-skills.md)

## 参考

- [命令参考](command-reference.md)
- [文件系统布局](filesystem-layout.md)
