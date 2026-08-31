# Import、Update 与 Export

语言：[English](../project-skills.md) | **简体中文**

sm 明确分离 inventory 修改与 target 投影：

```text
import/update/enable/disable -> profile inventory
apply                        -> persistent targets
```

这让外部 installer 可以与 sm 组合，而不需要拥有 `$SM_HOME`。

## Import 新 Skill

Import 把来源目录复制到一个 profile：

```console
sm import --profile project-tools --from .skills --create
```

规则：

- 除非显式使用 `--create`，目标 profile 必须已存在；
- 新建 profile 的 `.smtag` 为 `true`；
- import 可以增加 skills，但不删除来源目录；
- 已存在目标需要显式 `--replace`；
- replace 只影响本次选中名称；
- 来源中的 sm 受管链接会跳过；
- 其他来源软链接会被拒绝；
- 完整批次先 staging，再执行替换；
- import 永远不运行 apply。

选择一部分：

```console
sm import --profile matt \
  --from ~/.agents/skills \
  --create \
  --skill code-review \
  --skill tdd
```

若增加 skill 会让两个全局启用 profiles 出现同名项，import 会在修改前失败。先 disable 一个 owner，或导入到已停用 profile。

## Update 已有 Skill

`update` 按直接子目录名称搜索全部 profiles，并且只替换已有匹配：

```console
sm update --from ~/.agents/skills
```

未知来源目录保留原位，不会加入 sm。

一个名称在多个 profiles 中存在时，启用状态不会替你选择 owner。必须显式消歧：

```console
sm update --from ~/.agents/skills --profile matt
sm update --from ~/.agents/skills --all
```

`--profile` 只更新一个已存在 profile 中的匹配；`--all` 更新每个已有副本。两者都不能让 update 新建 profile 或 skill。

也可选择来源名称：

```console
sm update --from ~/.agents/skills --skill code-review --profile matt
```

显式选择的来源名称必须存在。整个 update 批次完成 staging 后才替换 inventory。

## npx 工作流

先让 installer 写入配置过的 target：

```console
npx skills@latest add mattpocock/skills
```

对于新 skills，只选择想管理的部分：

```console
sm profiles new matt
sm import --profile matt \
  --from ~/.agents/skills \
  --skill code-review \
  --skill tdd
sm apply --force
```

来源真实目录会一直保留到 force apply 删除，然后为全局启用 skills 重建链接。`--force` 也会删除 target 中不想保留的 installer 目录。

以后批量更新：

```console
npx skills@latest add mattpocock/skills
sm update --from ~/.agents/skills --profile matt
sm apply --force
```

这会更新已知 inventory，忽略新提供的未知 skills，最后恢复 target 链接。

## Export 项目副本

```console
sm export --profile common
sm export --skill research/web-search --to .agents/skills
```

至少需要一个 selector。后面的 selector 覆盖同名输出选择。已有 destination 永远不会被覆盖。导出副本归项目所有，sm 之后不会更新或删除。

## Dry Run

```console
sm import --profile matt --from ~/.agents/skills --create --dry-run
sm update --from ~/.agents/skills --dry-run
sm apply --force --dry-run
```

Dry run 只验证并输出 tab-separated operations，不创建 tag、inventory、链接或目录。
