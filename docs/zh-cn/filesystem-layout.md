# 文件系统布局

语言：[English](../filesystem-layout.md) | **简体中文**

sm 将 inventory、配置、进程协调状态、生成缓存和持久投影分开存放。

## Inventory

```text
${SM_HOME:-~/.sm}/
  profiles/
    <profile>/
      .smtag
      <skill>/
        SKILL.md
        ...
```

Profile 目录及其 `.smtag` 是 inventory 和 activation 的事实来源。该目录树可以由 Git 管理，但 sm 永远不会调用 Git。

缺少 `.smtag` 时会初始化为 `true`。Profile 中隐藏条目是 metadata，不是 skills。Skill 内部内容不透明。

## 配置

```text
${XDG_CONFIG_HOME:-~/.config}/sm/config.toml
```

配置包含命名 targets。每个 target 可以有持久 `skills_dir`、shell adapter，或同时具备两者。`sm adopt` 只编辑指定 target 的 `skills_dir`，并保留原 TOML 格式与注释。

## 本机状态

```text
${XDG_STATE_HOME:-~/.local/state}/sm/
  inventory/
    lock/
  targets/
    <target>/
      lock/
  leases/
    <generation-id>/
      <pid-nonce>/
```

Lock 只在操作执行期间存在。Inventory lock 串行化 tag/import/update 修改，target lock 串行化 apply。Lease 防止活跃 shell 或包装命令使用的 generation 被回收。

XDG state 中不再保存持久 activation。初始化 inventory 时会删除旧的 `targets/<target>/enabled/`。

## Cache

```text
${XDG_CACHE_HOME:-~/.cache}/sm/
  generations/
    <generation-id>/
      skills/
        <skill> -> $SM_HOME/profiles/<profile>/<skill>
      bin/
        <agent-wrapper>
      shell/
        bash/
        zsh/
```

Generation ID 代表所选 skill 路径、shell adapters、wrapper 可执行路径和 sm 可执行文件。Generation 创建后成员不变，但链接的 skill 内容仍可变化。

使用 `sm gc` 删除没有活跃 lease 的完整 generations。

## 持久 Targets

配置的 target 目录位于 sm state 之外：

```text
~/.agents/skills/
  code-review -> ~/.sm/profiles/matt/code-review
  web-search  -> ~/.sm/profiles/research/web-search
```

所有 targets 得到同一个全局启用集合。普通 apply 管理指向 `$SM_HOME/profiles` 的链接，并拒绝 skill 副本（含 `SKILL.md` 的真实目录）。Force apply 删除全部非隐藏 skill 副本，再安装 desired links；普通文件、隐藏条目、无关外部链接和不含 `SKILL.md` 的真实目录不受管理。

## 项目与 Installer 副本

Import、update 和 export 操作普通副本目录。Import 与 update 保留来源；如果来源本身就是 configured target，后续 force apply 可以删除其中的 skill 副本。
