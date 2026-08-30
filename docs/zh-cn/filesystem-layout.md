# 文件系统布局

语言：[English](../filesystem-layout.md) | **简体中文**

`sm` 将可同步的 skill 内容、用户配置、本机状态和可删除缓存分开存放。

## Skill 仓库

```text
${SM_HOME:-~/.sm}/
  profiles/
    <profile>/
      <skill>/
        SKILL.md
        ...
```

这是唯一预期通过 Git 同步的目录。`sm` 不要求它一定是 Git 仓库，也永远不会调用 Git。

## 配置

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

配置包含默认 target 和通用 target/template 数据。参见 [Targets 与模板](targets.md)。

## 本机状态

```text
${XDG_STATE_HOME:-~/.local/state}/sm/
  targets/
    <target>/
      enabled/
        <sequence>-<profile>/
      lock/
  profiles/
    <profile>/
      lock/
  leases/
    <generation-id>/
      ...
```

启用标记是空目录。每个 target 的 lock 用于串行化持久更新，每个 profile 的 lock 用于串行化对同一 profile 的 import。lock 目录只在操作运行期间存在。lease 条目阻止 `sm gc` 删除仍被活跃子 shell 使用的 generation。

状态只属于当前机器，不应提交到 skill 仓库。

## 缓存

```text
${XDG_CACHE_HOME:-~/.cache}/sm/
  generations/
    <generation-id>/
      skills/
        <skill> -> $SM_HOME/profiles/<profile>/<skill>
      bin/
        <configured-command-wrapper>
      shell/
        bash/
          bashrc
          bashenv
        zsh/
          .zshenv
          .zprofile
          .zshrc
          .zlogin
          .zlogout
```

generation ID 是不透明的实现细节。generation 创建后，其链接成员和目标不会改变。skill 内容仍可通过源符号链接改变。生成的 Bash 和 zsh 文件会 source 用户真实的启动文件，再恢复 command wrapper 的优先级；Fish 使用等效的 `--init-command`。这些文件是缓存数据，不是用户配置。

全部缓存都可以重建。正常清理请使用 `sm gc`；不要删除活跃子 shell 正在使用的 generation。

## 持久 Agent Targets

持久 target 目录不属于 `sm` 自身状态，而是在配置中声明。例如：

```text
~/.pi/agent/skills/
  code-review -> ~/.sm/profiles/coding/code-review
  web-search  -> ~/.sm/profiles/research/web-search
  manually-installed-skill/
```

`sm` 会保留非受管条目。它只删除目标指向 `$SM_HOME/profiles` 下 skill 的符号链接。

## 项目副本

默认 import/export 目录相对于当前项目：

```text
<project>/.skills/
  <skill>/
```

这些是普通复制目录，不属于 `sm` 状态。导出副本归项目所有，`sm` 不会继续同步或删除它们。
