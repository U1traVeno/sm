# Profiles 与全局启用

语言：[English](../profiles.md) | **简体中文**

## 布局

Profile 是 `$SM_HOME/profiles` 下的直接子目录：

```text
$SM_HOME/profiles/<profile>/
  .smtag
  <skill>/
    SKILL.md
```

Skill 是 profile 下非隐藏的直接子目录。sm 不解析 `SKILL.md`，并把 skill 内部内容视为不透明数据。Profile 和 skill 条目必须是真实目录，不能是软链接。

删除 profile 目录就同时删除了 profile 及其启用状态；手工新建目录就立即增加了 profile。

## `.smtag`

`.smtag` 只接受一个布尔值，可带结尾换行：

```text
true
```

或：

```text
false
```

`true` 表示 profile 属于全局启用集合。每个持久 target 都会得到同一个集合。

sm 发现 profile 缺少 `.smtag` 时，会创建内容为 `true` 的文件，并向标准错误输出一次初始化日志。Dry run 只报告该操作，不写文件。其他内容均视为错误。

显式创建 profile：

```console
sm profiles new matt
sm profiles new archive --disabled
```

修改全局启用状态：

```console
sm enable matt
sm disable archive
```

这些命令只修改 `.smtag`，不会修改 target。整理好 inventory 后再运行 `sm apply`。

## 同名 Skill

不同 profiles 可以库存同名 skill，但两个全局启用的 profiles 不能同时暴露同名项：

```text
profiles/old/code-review/
profiles/matt/code-review/
```

只要最多有一个 owner 启用，这个 inventory 就有效。`sm enable`、向已启用 profile 执行 import，以及 `sm apply` 都会在修改前拒绝启用冲突。

隔离 shell 可以临时显式选择含同名 skill 的 profiles。此时 selector 从左到右解析，后者胜出，但不会持久化优先级。

## 手工编辑

文件系统是事实来源。用户可以直接：

- 创建或删除 profile 目录；
- 在 profiles 之间移动 skill；
- 编辑 `.smtag`；
- 原地编辑 skill 内容。

成员或启用状态改变后运行 `sm apply`。外部 installer 产生了已有 skill 的新版本时，运行 `sm update --from DIRECTORY`。

## 旧启用状态

旧版本在 XDG state 下保存 per-target enabled marker。首次非 dry-run inventory 命令会删除这些过时目录；全局启用模型永远不会读取它们。
