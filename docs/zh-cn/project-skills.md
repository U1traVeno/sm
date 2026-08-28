# 项目 Skill 的导入与导出

语言：[English](../project-skills.md) | **简体中文**

项目传输使用目录复制。它不同于使用符号链接的持久激活和隔离 shell generation。

复制是一次性的所有权转移：

- export 后由项目拥有副本；
- import 后由目标 profile 拥有副本；
- `sm` 不会继续跟踪、更新、同步或删除这些副本。

## 导出 Profiles

导出一个或多个完整 profile：

```console
$ sm export --profile common --profile coding
```

默认目的地是当前工作目录下的 `.skills/`。

也可以显式导出到 Agent 能识别的项目目录：

```console
$ sm export --profile coding --to .agents/skills
```

`.skills/` 是中立的 vendored storage。`sm` 不假设 Agent 会自动发现它。

## 导出单个 Skills

使用带 profile 限定的引用：

```console
$ sm export --skill research/web-search
```

完整 profile 和单个 skill 可以组合：

```console
$ sm export \
  --profile common \
  --skill research/web-search \
  --to .skills
```

selectors 形成并集，并按从左到右的顺序解析。多个已选 profile 中存在同名 skill 时，后出现的 selector 胜出。

必须至少提供一个 `--profile` 或 `--skill`。export 永远不会继承 target 的已启用 profiles。

## 导入到 Profile

把 `.skills/` 下所有直接 skill 子目录导入一个 profile：

```console
$ sm import --profile project-tools
```

也可以只导入指定 skills：

```console
$ sm import \
  --profile project-tools \
  --skill deploy \
  --skill release-notes
```

使用 `--from` 指定其他源目录：

```console
$ sm import --profile project-tools --from .agents/skills
```

目标 profile 不存在时会创建。导入后的 skills 是以下位置中的普通目录：

```text
$SM_HOME/profiles/<profile>/<skill>/
```

## 冲突规则

复制任何内容前，`sm` 会校验全部源名称和目标名称。如果任意目标条目已经存在，整个操作失败且不会复制任何内容。

`sm` 不会：

- 合并目录；
- 覆盖已有文件；
- 比较版本；
- 更新早先的副本；
- 提供 force 参数。

请使用 `diff -r`、`rm -r` 和 `mv` 等普通工具解决冲突，然后重新运行命令。

## 复制语义

`sm` 递归复制每个已选 skill 目录，并保留可执行权限位和 skill 内部的符号链接。目标中的顶层 skill 是真实目录。

import/export 成功时保持静默。使用 `--dry-run` 查看计划：

```console
$ sm export --profile common --dry-run
copy	/Users/me/.sm/profiles/common/code-review	/working/project/.skills/code-review
```
