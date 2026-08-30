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

显式替换选中的已有 skill 目录：

```console
$ sm import --profile project-tools --from .agents/skills --replace
```

目标 profile 不存在时会创建。导入后的 skills 是以下位置中的普通目录：

```text
$SM_HOME/profiles/<profile>/<skill>/
```

## 与包安装器组合

`sm` 不调用包安装器。安装器可以先写入临时项目目录，再执行 `sm import --replace`。下面的 zsh 函数把 `npx skills` 与默认持久 target 组合起来：

```zsh
sm-add() {
  if (( $# < 2 )); then
    print -u2 'usage: sm-add PROFILE SOURCE [SKILLS-ADD-OPTION...]'
    return 2
  fi

  local profile=$1 arg tmp
  shift
  for arg in "$@"; do
    case $arg in
      -g|--global|--global=*|-a|--agent|--agent=*|--all|--copy)
        print -u2 "sm-add: unsupported skills add option: $arg"
        return 2
        ;;
    esac
  done

  tmp=$(mktemp -d "${TMPDIR:-/tmp}/sm-add.XXXXXXXX") || return 1
  {
    (
      cd "$tmp" &&
        command npx --yes skills add "$@" --agent universal --yes
    ) || return
    command sm import \
      --profile "$profile" \
      --from "$tmp/.agents/skills" \
      --replace || return
    if ! command sm apply; then
      print -u2 "sm-add: imported into profile $profile, but sm apply failed"
      return 1
    fi
  } always {
    command rm -rf -- "$tmp"
  }
}
```

第一个参数属于 `sm`，其余参数传给 `skills add`。函数拒绝 scope、agent、全 agent 和 copy 选项，因为这些选项可能绕过临时规范目录。需要 apply 到非默认 target 时设置 `SM_TARGET`：

```console
$ SM_TARGET=pi sm-add development vercel-labs/agent-skills --skill web-design-guidelines
```

该函数导入库存，但不会启用 profile 或改变优先级。第一次使用时应显式运行 `sm enable <profile>`。临时 `skills-lock.json` 会被丢弃，因此更新时重新执行原始 `sm-add` 命令，而不是使用 `npx skills update`。如果 import 成功而 apply 失败，导入的 profile 会保留；解决 target 问题后再次运行 `sm apply`。

## 冲突规则

复制任何内容前，`sm` 会校验全部源名称和目标名称。未指定 `--replace` 时，任意目标条目已经存在都会使整个操作失败且不复制内容。

指定 `--replace` 时，每个选中的已有目标都必须是实体目录。目标变化开始前，新内容会全部复制到 staging。安装过程中的普通错误会回滚选中的名称。替换不会合并目录、删除源中未出现的 profile 条目、比较版本或同步早先的副本。

需要比较、合并或删除语义时，请使用 `diff -r`、`rm -r` 等普通工具。

## 复制语义

`sm` 递归复制每个已选 skill 目录，并保留可执行权限位和 skill 内部的符号链接。目标中的顶层 skill 是真实目录。

import/export 成功时保持静默。使用 `--dry-run` 查看计划：

```console
$ sm export --profile common --dry-run
copy	/Users/me/.sm/profiles/common/code-review	/working/project/.skills/code-review
```
