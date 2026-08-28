# Profiles 与激活

语言：[English](../profiles.md) | **简体中文**

## Profile 布局

profile 是 `$SM_HOME/profiles` 下的目录：

```text
$SM_HOME/profiles/<profile>/<skill>/
```

只有直接的、非隐藏子目录会被视为 skills。profile 不能使用普通文件或符号链接表达 membership。每个 skill 目录下面的内容对 `sm` 是不透明的。

例如：

```text
~/.sm/profiles/
  common/
    web-search/
    code-review/
  backend/
    database-migrations/
    code-review/
```

如果一个 skill 在概念上属于多个分组，应将它放进单独的 profile，例如 `common`。`sm` 不维护中央 skill 目录，也不管理 profile membership 链接。

## 启用 Profiles

每个 target 独立维护已启用 profiles：

```console
$ sm enable common backend --target pi
```

参数按从左到右的顺序处理。后启用的 profile 优先级更高。重新启用已启用的 profile 会把它提升到最高优先级，不会产生重复项。

`sm enable` 执行以下步骤：

1. 校验所有指定的 profile 及其直接 skill 子目录。
2. 计算全部已启用 profiles 的并集。
3. 检查 target 中是否存在非受管冲突。
4. 使用文件系统标记目录记录新的优先级。
5. 将最终胜出的 skills 物化为符号链接。

可预期的校验错误会在 target 发生变化前失败。

## 禁用 Profiles

```console
$ sm disable backend --target pi
```

禁用会从该 target 的启用集合中移除 profile，并重新物化剩余并集。该命令是幂等的：禁用一个已经禁用的 profile 仍会协调 target，并静默成功。

## Skill 重名

直接子目录名就是 skill identity。`sm` 不比较 `SKILL.md` frontmatter，也不比较文件内容。

给定以下启用顺序：

```text
common
backend
```

以及以下目录：

```text
profiles/common/code-review/
profiles/backend/code-review/
```

因为 `backend` 后启用，所以 `backend/code-review` 胜出。禁用 `backend` 后，`common/code-review` 会重新变为可见。

## 受管与非受管条目

只有当符号链接保存的目标指向 `$SM_HOME/profiles/<profile>/<skill>` 下的 skill 时，`sm` 才会管理它。

`sm` 不会删除或替换：

- 普通文件；
- 普通目录；
- 指向 `$SM_HOME/profiles` 以外的符号链接；
- 无关的断裂符号链接。

如果非受管条目与选中 skill 同名，操作会在修改 target 前失败。请显式解决冲突后重试。

## 本机状态

启用状态使用目录而不是 manifest：

```text
~/.local/state/sm/targets/pi/enabled/
  00000000000000000001-common/
  00000000000000000002-backend/
```

数字前缀记录优先级，标记目录内部为空。重新启用 profile 时，旧标记会被一个序号更高的新标记替换。

这些状态仅属于当前机器，不应提交到 skill 仓库。

## 同步仓库

Git 操作单独执行：

```console
$ git -C ~/.sm pull --ff-only
$ sm enable common --target pi
```

重新启用会协调 target，并将 `common` 提升到最高优先级。如果只想协调而不改变优先级，请运行：

```console
$ sm apply --target pi
```

`sm apply` 保留现有标记顺序，并修复受管链接。
