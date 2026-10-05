# 快速开始

语言：[English](../getting-started.md) | **简体中文**

## 1. 安装

```console
cargo install sm-skill-manager
```

该 crate 安装 `sm`，要求 Rust 1.88 或更新版本。

## 2. 创建 Profile

```console
sm profiles new common
```

这会创建：

```text
~/.sm/profiles/common/.smtag
```

内容为 `true`。手工创建的 profile 目录也有效；sm 下次读取 inventory 时会把缺少的 `.smtag` 初始化为 `true`。

直接增加 skill 目录，或者导入：

```console
sm import --profile common --from .skills
```

## 3. 配置 Target

登记一个持久投影目录：

```console
sm adopt agents ~/.agents/skills
```

也可以编辑 `~/.config/sm/config.toml`：

```toml
[targets.agents]
skills_dir = "~/.agents/skills"
```

Target 不拥有 activation。每个 target 得到同一个全局集合。

## 4. Apply

```console
sm apply
```

该命令为所有 `.smtag` 为 `true` 的 profile skills 创建软链接。

若 installer 已在 target 写入真实目录，先检查：

```console
sm apply --force --dry-run
```

然后收敛：

```console
sm apply --force
```

Force 会删除选中 targets 内全部非隐藏 skill 副本（含 `SKILL.md` 的真实目录）。普通文件、隐藏条目、无关外部软链接和不含 `SKILL.md` 的真实目录会保留。

## 5. 启用与停用

```console
sm disable common
sm apply

sm enable common
sm apply
```

Enable 和 disable 只更新 `.smtag`；apply 始终显式执行。

## 6. 增加 Installer Skill

外部 installer 写入 target 后，只导入需要的新 skills：

```console
sm profiles new matt
sm import --profile matt \
  --from ~/.agents/skills \
  --skill code-review \
  --skill tdd
sm apply --force
```

Import 保留来源目录。Force apply 删除 installer 目录并恢复 desired symlinks。

## 7. 更新已管理 Skill

先广泛下载，再只更新 sm 中已经存在的名称：

```console
npx skills@latest add mattpocock/skills
sm update --from ~/.agents/skills --profile matt
sm apply --force
```

Update 永远不会加入未知下载。

## 8. 配置隔离 Shell

增加 adapter：

```toml
[targets.pi.shell]
command = "pi"
args = ["--no-skills", "--skill", "{skills}"]
```

启动临时 profile scope：

```console
sm shell --profile matt
(sm) $ pi
```

未显式选择 profile 时，shell generation 继承全局集合。未限制 target 时，所有 adapters 都会被包装。

## 9. 检查状态

```console
sm profiles
sm skills
sm enabled
sm targets
sm status
```

在 mutating commands 上使用 `--dry-run`，可验证计划而不修改文件。
