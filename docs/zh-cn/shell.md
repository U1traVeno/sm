# 隔离 Shell

语言：[English](../shell.md) | **简体中文**

`sm shell` 启动一个交互式子 shell，使其中一个已配置命令使用稳定的、会话专属的 skill 集合。

它不会修改父 shell，也不会替换固定的全局 skill 目录。

## 基本用法

```console
$ sm shell pi --profile common --profile research
(sm) $ pi
```

子 shell 继承当前工作目录和普通环境。`sm` 会在 `PATH` 前添加一个私有 wrapper 目录。只有已配置命令会被包装，其他命令行为不变。

`sm` 会根据检测到的 shell 采用对应启动策略：

- **zsh：**generation 专属的 `ZDOTDIR` shim 会 source 用户原有的启动文件、恢复 wrapper 优先级，并在执行命令前恢复原来的 `ZDOTDIR` 状态。
- **bash：**交互式非 login shell 使用生成的 `--rcfile` source 用户 `.bashrc`；非交互命令通过生成的 `BASH_ENV` 做相同处理，之后恢复用户原有的 `BASH_ENV` 状态。
- **fish：**使用 `--init-command`，在 `config.fish` 执行后恢复 wrapper 优先级。

这些策略会保留 aliases、functions、options 和环境变更，用户启动文件无需加入 sm 专用代码。Bash login shell（`-l` 或 `--login`）会被拒绝，因为 Bash 不会为它读取 `--rcfile`；请使用默认的非 login 子 shell。

其他 shell 会继承 wrapper 优先的 `PATH`；它们的启动文件不得把同名可执行文件放到 wrapper 前面。

退出子 shell 即可结束作用域：

```console
(sm) $ exit
$
```

已经从子 shell 启动的 Agent 会继续使用其 generation，直到 Agent 退出。

## Profile 选择

只要出现至少一个 `--profile`，profile 集合就是显式集合，不继承持久启用状态：

```console
$ sm shell pi --profile common --profile coding
```

没有 `--profile` 时，`sm shell` 继承该 target 的持久启用 profiles：

```console
$ sm shell pi
```

可以用带 profile 限定的引用添加单个 skill：

```console
$ sm shell pi --skill research/web-search
```

没有 `--profile` 时，它会在继承集合上增加 `research/web-search`。存在一个或多个 `--profile` 时，它会加入显式集合。

selector 从左到右解析。重名时，后出现的 selector 胜出。

## 稳定 Generations

generation 是存储在缓存目录中的符号链接映射：

```text
~/.cache/sm/generations/<generation-id>/
  skills/
    code-review -> ~/.sm/profiles/coding/code-review
    web-search  -> ~/.sm/profiles/research/web-search
```

generation ID 表示已解析的源路径和优先级。创建完成后，其成员和链接目标不会原地修改。不同的解析结果会创建或复用另一个 generation。

因此，两个子 shell 不会覆盖彼此可见的 skill membership。之后改变持久启用 profile，也不会使既有 generation 路径失效。

skill 内容本身并非不可变。generation 使用符号链接，因此编辑或删除源 skill 会改变既有 generation 所看到的内容。当运行中的 Agent 需要可复现 skill 正文时，不要改写 skill 仓库。

## 与 Agent 解耦

`sm shell` 并非 Pi 专用。它使用 [Targets 与模板](targets.md) 中的通用命令模板。

只有能够通过 argv 或环境变量接收 skill 目录的命令才支持隔离 shell。如果命令只能扫描固定全局目录，`sm shell` 会针对该 target 失败；请改用持久 `enable` 和 `disable`。

## 垃圾回收

子 shell 存活期间，`sm` 会持有 generation lease。`sm gc` 只删除没有活跃 lease 的 generation：

```console
$ sm gc --dry-run
remove	/Users/me/.cache/sm/generations/old-id
$ sm gc
```

垃圾回收只会删除完整 generation 目录，不会原地编辑 generation。

## 失败行为

以下情况会在子 shell 启动前失败：

- target 未知；
- target 没有 shell 模板；
- 选中的 profile 或 skill 不存在；
- 重名解析在 generation 路径中产生非受管冲突；
- 配置的命令无法解析；
- generation 创建不完整。
