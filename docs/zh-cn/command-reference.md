# 命令参考

语言：[English](../command-reference.md) | **简体中文**

## 通用约定

- `-t` 是 `--target` 的短参数。
- 配置中允许路径时，以 `~` 开头的路径会展开。
- profile 名和 target 名都是单个目录组件。
- skill 引用格式是 `<profile>/<skill>`。
- 除非使用 `--dry-run`，修改型命令成功时保持静默。
- 主要输出写入标准输出，诊断信息写入标准错误。

## `sm profiles`

按字节序逐行列出 profile 名称。

```text
sm profiles
```

## `sm skills`

列出带 profile 限定的 skill 名称。

```text
sm skills [PROFILE...]
```

不提供 profile 参数时，列出所有 profile 的 skills。每行输出一个 `<profile>/<skill>`，按字节序排序。

## `sm targets`

逐行列出已配置 target 名称。

```text
sm targets
```

## `sm enabled`

按优先级从低到高输出某个持久 target 已启用的 profiles。

```text
sm enabled [-t TARGET]
```

## `sm status`

输出某个持久 target 当前实际物化的胜出 skills。

```text
sm status [-t TARGET]
```

每行以制表符分隔：

```text
<skill-name>\t<absolute-source>\t<absolute-destination>
```

空 target 不产生输出并成功退出。

## `sm enable`

为一个持久 target 启用 profiles。

```text
sm enable PROFILE... [-t TARGET] [--dry-run]
```

profiles 从左到右处理。每个指定 profile 都会移动到最高优先级。`sm` 会预检最终并集，然后将其物化为受管符号链接。

## `sm disable`

为一个持久 target 禁用 profiles。

```text
sm disable PROFILE... [-t TARGET] [--dry-run]
```

profiles 从左到右处理。禁用已经禁用的 profile 仍然成功，并会协调 target。

## `sm apply`

在不改变已启用 profiles 和优先级的情况下协调一个持久 target。

```text
sm apply [-t TARGET] [--dry-run]
```

修改或更新 profile 目录后，或需要修复中断的 target 更新时使用该命令。

## `sm shell`

使用 target 的通用 shell 模板启动隔离子 shell。

```text
sm shell TARGET \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [-- SHELL-ARGUMENT...]
```

选择行为：

- 出现任意 `--profile` 时，只使用显式选择的 profiles 和 `--skill` 附加项；
- 没有 `--profile` 时，继承 target 的持久启用 profiles，再加入 `--skill`。

`--` 后的参数传给子 shell，而不是被包装的 Agent 命令。

## `sm export`

把 profiles 或单个 skills 复制到项目拥有的目录。

```text
sm export \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [--to DIRECTORY] \
  [--dry-run]
```

必须至少提供一个 selector。默认目的地是 `.skills/`。任意目标名称已存在都会使整个操作失败。

## `sm import`

把项目拥有的 skills 复制进一个 profile。

```text
sm import \
  --profile PROFILE \
  [--skill SKILL]... \
  [--from DIRECTORY] \
  [--replace] \
  [--dry-run]
```

默认来源是 `.skills/`。没有 `--skill` 时导入全部直接 skill 子目录。目标 profile 不存在时会创建。任意目标名称已存在都会使整个操作失败，除非显式指定 `--replace`。替换只影响选中的名称，要求已有目标是实体目录，并保留 profile 中的其他所有条目。

## `sm gc`

删除没有 lease 的 shell generations。

```text
sm gc [--dry-run]
```

正在使用的 generation 永远不会被删除。

## Dry-Run 输出

`--dry-run` 会执行校验并输出计划操作，但不修改内容。输出以制表符分隔，每行一个操作。操作名称使用小写：

```text
enable\t<TARGET>\t<PROFILE>
disable\t<TARGET>\t<PROFILE>
link\t<SOURCE>\t<DESTINATION>
unlink\t<DESTINATION>
copy\t<SOURCE>\t<DESTINATION>
replace\t<SOURCE>\t<DESTINATION>
remove\t<PATH>
```

路径使用绝对路径。

## 退出状态

- `0`：成功，包括幂等的无操作情况。
- `1`：运行失败，例如 profile 不存在、发生冲突、target 无效或复制错误。
- `2`：命令行用法错误。

可预期的失败不会修改 target 或目的地。操作系统级中断可能留下只协调了一部分的持久 target，或隐藏的 import staging/backup 目录。target 更新可重新运行 `sm apply`；删除中断的 import 数据前应先检查对应 profile。
