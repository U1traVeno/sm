# Targets 与模板

语言：[English](../targets.md) | **简体中文**

一个 target 可以描述两个可选能力：

1. `sm enable`、`sm disable` 和 `sm apply` 使用的持久 skill 目录。
2. `sm shell` 使用的通用命令模板。

核心中不包含针对具体 Agent 的条件分支。Pi target、未来的 Agent target 和用户自定义命令都使用相同字段。

## 配置文件

默认配置路径是：

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

示例：

```toml
default_target = "pi"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]

[targets.example]
skills_dir = "~/.example/skills"

[targets.example.shell]
command = "/usr/local/bin/example-agent"
args = ["--skills-dir", "{skills}"]
env = { EXAMPLE_MODE = "isolated" }
```

## Target 选择

命令按以下顺序选择 target：

1. `--target <name>`
2. `SM_TARGET`
3. `config.toml` 中的 `default_target`
4. 当且仅当只配置了一个 target 时，使用该 target

如果仍然无法确定，命令会失败，并在标准错误中列出可用 target 名称。

`sm shell <target>` 始终显式接收 target 名称。

## 持久能力

`skills_dir` 是持久激活链接的物化目录。`~` 会展开为用户家目录。相对路径会被拒绝。

缺少 `skills_dir` 的 target 不能用于：

- `enable`
- `disable`
- `apply`
- `status`

## Shell 能力

shell 配置接受：

- `command`：可执行文件名或绝对路径；
- `args`：插入到用户参数之前的 argv 项；
- `env`：wrapper 设置的可选环境变量。

`{skills}` 会展开为稳定 generation 中 `skills/` 目录的绝对路径。它必须至少出现在一个 `args` 或 `env` 值中。

模板是数据，不是 shell 命令。`sm` 不进行 shell 插值、分词、命令替换或 `eval`。

启动子 shell 前，`sm` 会使用原始 `PATH` 解析 `command`，然后在私有目录中创建同名 wrapper，并把该目录放在子 shell 的 `PATH` 最前面。wrapper 使用已解析命令、配置参数、配置环境和用户输入参数执行真实命令。

zsh、Bash 和 Fish 子 shell 会使用各自的启动适配，在用户配置执行后恢复 wrapper 优先级：zsh 使用生成的 `ZDOTDIR`，Bash 使用 `--rcfile`/`BASH_ENV`，Fish 使用 `--init-command`。Bash login shell 不读取 `--rcfile`，因此会被拒绝。其他 shell 要求其启动文件保留继承的 wrapper-first PATH。

对于 Pi 示例，在子 shell 中输入：

```console
(sm) $ pi --model sonnet
```

等价于以下 argv：

```text
/path/to/pi --no-skills --skill /absolute/generation/skills --model sonnet
```

## 内置模板

发行包可以附带经过验证的 target 模板。内置模板与用户配置使用同一 schema，不拥有任何核心特权。用户配置同名 target 时会覆盖内置数据。

只有当命令能通过 argv 或环境变量接收显式 skill 目录时，模板才能声明 shell 支持。只能扫描固定全局目录的命令仅支持持久激活。

## 安全性

配置错误会在 shell 启动或持久 target 修改前失败。`sm` 会拒绝：

- 未知 target 名；
- 相对持久路径；
- 不含 `{skills}` 的 shell 模板；
- 无法解析的命令；
- 格式错误的 argv 或环境变量值。
