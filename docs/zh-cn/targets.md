# Targets 与 Shell Adapters

语言：[English](../targets.md) | **简体中文**

一个 target 可以描述以下一种或两种能力：

1. `sm apply` 投影全局启用 skill 集合的持久目录；
2. 能让 agent 命令使用显式 generation 目录的 shell adapter。

Target 不选择 profiles，也不保存启用状态。

## 配置

默认配置路径：

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

示例：

```toml
[targets.agents]
skills_dir = "~/.agents/skills"

[targets.pi]
skills_dir = "~/.pi/agent/skills"

[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

`skills_dir` 必须解析为绝对路径。目录可以暂时不存在，`sm apply` 会创建它。两个 targets 不能使用同一路径，target 也不能与 `$SM_HOME/profiles` 存在任一方向的路径重叠。

旧的 `default_target` 字段仍可解析以保持兼容，但不再影响 activation 或 apply 的默认范围。

## 登记 Target

```console
sm adopt agents ~/.agents/skills
```

`adopt` 给指定 target 增加 `skills_dir`，同时保留 TOML 格式、注释和已有 shell 配置。相同名称与规范化路径的重复调用是幂等成功；名称或路径重映射会被拒绝。

`adopt` 不创建目录、不推断 agent 类型、不增加 shell adapter，也不运行 apply。

## 持久投影

```console
sm apply
```

未指定 target 时，sm 会先 preflight 所有带 `skills_dir` 的 targets，再修改任何一个。它们得到完全相同的 desired links。

```console
sm apply --target agents
```

该选项只缩小本次修复范围，不会产生 target 专属启用状态。

普通 apply：

- 修复缺失或陈旧的 sm 受管链接；
- 删除不再需要的 sm 受管链接；
- 保留隐藏条目、普通文件、外部软链接和不含 `SKILL.md` 的真实目录；
- 发现任意非隐藏 skill 副本（含 `SKILL.md` 的真实目录）就整体失败；
- 被保留条目占用 desired skill 名称时失败。

`sm apply --force` 会先删除选中 targets 内全部非隐藏 skill 副本，再创建链接。普通文件、隐藏条目、不相关的外部软链接，以及不含 `SKILL.md` 的真实目录（属于其他程序）仍会保留。

## Shell Adapter

Shell adapter 是结构化数据，不是 shell 源码：

```toml
[targets.example.shell]
command = "/absolute/path/to/example-agent"
args = ["--skills-dir", "{skills}"]
env = { EXAMPLE_MODE = "isolated" }
```

`{skills}` 必须出现在 `args` 或 `env`。sm 在进入子 shell 前解析命令，并创建同名 wrapper。整个过程不使用 `eval`、word splitting 或 command substitution。

只能读取固定全局目录的 agent 无法被隔离。不要为它配置 adapter；它在 `sm shell` 内仍会看到全局投影。
