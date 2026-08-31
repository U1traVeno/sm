# 命令参考

语言：[English](../command-reference.md) | **简体中文**

## 通用约定

- Profile、skill 和 target 名称是一个非隐藏目录组件。
- Skill 引用格式为 `<profile>/<skill>`。
- 全局 activation 来自 `<profile>/.smtag`。
- 除一次性 migration 日志外，成功修改默认静默。
- `--dry-run` 只验证并输出操作。
- `-t` 是 `--target`，`-f` 是 `--force`。

## `sm profiles`

```text
sm profiles
sm profiles new PROFILE... [--disabled] [--dry-run]
```

列出 profiles 或创建 profiles。新 profile 默认启用。

## `sm skills`

```text
sm skills [PROFILE...]
```

输出 `<profile>/<skill>`。无参数时检查全部 profiles。

## `sm targets`

```text
sm targets
```

列出 configured targets。

## `sm enabled`

```text
sm enabled
```

按词法顺序列出全局启用 profiles。

## `sm status`

```text
sm status [-t TARGET]
```

未指定 target 时每行格式为：

```text
<target>\t<skill>\t<source>\t<destination>
```

指定 target 时省略第一列。

## `sm enable` 与 `sm disable`

```text
sm enable PROFILE... [--dry-run]
sm disable PROFILE... [--dry-run]
```

向 profile tags 写入 `true` 或 `false`，不修改 targets。启用后产生同名冲突时会在写入前失败。

## `sm apply`

```text
sm apply [-t TARGET] [-f|--force] [--dry-run]
```

将全局启用 skills 投影到全部或一个 target。修改前会 preflight 所有选中 targets。

无 force 时，任意非隐藏真实目录都会阻止整个操作。使用 force 时，这些目录会删除。受管链接按需修复或删除。普通文件、隐藏条目和无关外部链接会保留，除非占用 desired 名称。

## `sm shell`

```text
sm shell \
  [--target TARGET]... \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [-- SHELL_ARGUMENT...]
```

使用一个稳定 generation 启动子 shell。不限制 target 时包装所有 configured adapters。不指定 profile 时继承全局集合；出现任意 profile 时改用显式集合。

## `sm import`

```text
sm import \
  --profile PROFILE \
  [--skill SKILL]... \
  [--from DIRECTORY] \
  [--create] [--replace] [--dry-run]
```

把来源目录复制到一个 profile。默认来源为 `.skills`。Import 可以增加 skill 并显式新建 profile，但保留全部来源目录。普通非目录条目会忽略，sm 受管来源链接会跳过，其他软链接会被拒绝。

## `sm update`

```text
sm update \
  --from DIRECTORY \
  [--skill SKILL]... \
  [--profile PROFILE | --all] \
  [--dry-run]
```

只替换 inventory 中已有的同名 skills。未知来源目录和普通非目录条目会跳过，永远不会 import。同名 inventory 冲突必须指定一个 profile 或全部 owners。来源目录保留。

## `sm export`

```text
sm export \
  [--profile PROFILE]... \
  [--skill PROFILE/SKILL]... \
  [--to DIRECTORY] [--dry-run]
```

把选中的 inventory skills 复制到项目目录。至少需要一个 selector，已有 destination 永远不会覆盖。

## `sm adopt`

```text
sm adopt TARGET DIRECTORY [--dry-run]
```

为 target 登记规范化绝对 `skills_dir`，同时保留 TOML 格式与 shell 配置。Adopt 不创建目录，也不运行 apply。

## `sm gc`

```text
sm gc [--dry-run]
```

删除没有活跃 lease 的隔离 shell generations。

## Dry-Run 输出

输出为 tab-separated，可能包含：

```text
tag\ttrue\t<path>
create-profile\t<enabled>\t<path>
enable\t<profile>
disable\t<profile>
copy\t<source>\t<destination>
replace\t<source>\t<destination>
update\t<source>\t<destination>
skip\t<source>
adopt\t<target>\t<directory>
link\t<source>\t<destination>
unlink\t<destination>
remove\t<path>
```

## 退出状态

- `0`：成功，包括幂等 no-op；
- `1`：操作或验证失败；
- `2`：命令行用法错误。

预期验证错误发生在 inventory 或 selected targets 修改前。若 OS 中断导致部分链接已变更，重新运行 `sm apply` 即可收敛。
