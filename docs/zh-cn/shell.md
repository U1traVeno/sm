# 隔离 Shell

语言：[English](../shell.md) | **简体中文**

`sm shell` 启动一个子命令 shell，让配置过的 agent 命令使用稳定、会话专属的 skill generation。它不会修改持久 targets。

## 基本使用

```console
sm shell --profile matt
(sm) $ pi
(sm) $ other-agent
```

没有 `--target` 时，所有配置过的 shell adapters 都会被包装。需要时可以限制：

```console
sm shell --target pi --profile matt
```

两个被选 adapter 不能使用同一个 command basename。发生冲突时显式选择一个 target。

## Skill 选择

没有 `--profile` 时，generation 继承全局启用 profiles：

```console
sm shell
```

出现任何显式 profile 后，只使用显式集合：

```console
sm shell --profile common --profile matt
```

可以追加单个 profile-qualified skill：

```console
sm shell --skill research/web-search
```

没有显式 profile 时，它追加到全局集合；否则追加到隔离集合。Selectors 从左到右解析，后者可以临时覆盖同名 skill。

## 子 Shell 行为

子 shell 继承当前工作目录和普通环境。`--` 后的参数传给子 shell：

```console
sm shell --target pi -- -c 'pi --model sonnet'
```

sm 把 generation wrappers 放在 `PATH` 首位，并设置：

```text
SM_SKILLS_DIR=<generation>/skills
SM_GENERATION=<generation-id>
```

只有配置过的 command basename 会被包装，其他命令不受影响。没有 adapter 的 agent 继续使用其普通全局行为。

## 启动文件

- **zsh：** 生成的 shim 会 source 用户真实启动文件，再恢复 wrapper 的 PATH 优先级。
- **Bash：** 生成的 `--rcfile` 和 `BASH_ENV` 保留用户启动状态并恢复优先级。Login shell 会被拒绝，因为它绕过 `--rcfile`。
- **Fish：** 使用 `--init-command` 在 `config.fish` 后恢复优先级。
- **其他 shell：** 接收 wrapper-first PATH，其启动文件必须自行保留该顺序。

## Generations 与 Leases

```text
~/.cache/sm/generations/<id>/
  skills/
    code-review -> ~/.sm/profiles/matt/code-review
  bin/
    pi
    other-agent
```

Generation 创建后不原地改变成员或 wrapper 集合，因此多个子 shell 可以并发使用不同 skills。活跃子 shell 和包装命令通过 PID lease 防止 generation 被回收。

清理没有 lease 的 generation：

```console
sm gc
```
