# 快速开始

语言：[English](../getting-started.md) | **简体中文**

## 环境要求

当前实现支持 macOS 和 Linux。从源码构建需要 Rust 1.88 或更高版本。

持久激活和 shell generation 需要文件系统支持创建符号链接。项目导入和导出使用复制，不使用符号链接。

运行 `sm` 时不要求 Git 存在。如果 `~/.sm` 是 Git checkout，请使用普通 Git 命令同步。

## 安装

从 crates.io 安装：

```console
$ cargo install sm-skill-manager
```

crate 名称是 `sm-skill-manager`，安装后的可执行文件仍是 `sm`。若要从项目 checkout 安装：

```console
$ cargo install --path .
```

## 创建 Skill 仓库

`SM_HOME` 默认为 `~/.sm`。profile 是 `profiles/` 的直接子目录，skill 是 profile 下的直接子目录：

```text
~/.sm/
  profiles/
    common/
      shell-tools/
        SKILL.md
    coding/
      code-review/
        SKILL.md
      repository-search/
        SKILL.md
```

`sm` 使用目录名识别 profile 和 skill，不解析或校验 `SKILL.md`。

若要在多台机器间同步全部 skills，可让 `~/.sm` 成为普通 Git checkout：

```console
$ git clone git@example.com:you/skills.git ~/.sm
$ git -C ~/.sm pull --ff-only
```

`sm` 自身永远不会执行这些命令。

## 配置 Target

创建 `~/.config/sm/config.toml`：

```toml
default_target = "pi"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

持久 target 和 shell 模板是两个独立能力：

- `skills_dir` 允许使用 `enable` 和 `disable`。
- `shell` 允许使用隔离的 `sm shell` 会话。
- 一个 target 可以只定义其中一种能力，也可以同时定义两种。

完整格式见 [Targets 与模板](targets.md)。

## 激活 Profiles

```console
$ sm enable common coding
$ sm enabled
common
coding
```

`sm enabled` 按优先级从低到高输出。重新启用已启用的 profile，会把它移动到末尾：

```console
$ sm enable common
$ sm enabled
coding
common
```

如果两个 profile 都有 `code-review`，此时 `common` 下的版本可见。

禁用一个 profile 不会影响其他 profile：

```console
$ sm disable common
```

未配置默认 target 或需要操作其他 target 时，使用 `--target`：

```console
$ sm enable research --target claude
```

## 查看状态

以下命令提供逐行输出：

```console
$ sm profiles
coding
common
research

$ sm skills coding
coding/code-review
coding/repository-search

$ sm targets
claude
pi
```

查看某个 target 中实际生效的 skills：

```console
$ sm status --target pi
code-review	/Users/me/.sm/profiles/coding/code-review	/Users/me/.pi/agent/skills/code-review
```

实际输出使用绝对路径，字段之间以制表符分隔。

## 后续阅读

- [Profiles 与激活](profiles.md)：优先级与冲突行为。
- [隔离 Shell](shell.md)：让并发 Agent 使用不同 skill 集合。
- [项目 Skill 的导入与导出](project-skills.md)：把 skills 放进项目仓库。
