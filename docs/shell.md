# Isolated Shells

Language: **English** | [简体中文](zh-cn/shell.md)

`sm shell` starts a child command shell where configured agent commands use a stable, session-specific skill generation. It never modifies persistent targets.

## Basic Use

```console
sm shell --profile matt
(sm) $ pi
(sm) $ other-agent
```

With no `--target`, every configured shell adapter is wrapped. Limit wrappers when needed:

```console
sm shell --target pi --profile matt
```

Two selected adapters may not use the same command basename. Select one target explicitly to resolve such a conflict.

## Skill Selection

Without `--profile`, the generation inherits globally enabled profiles:

```console
sm shell
```

Any explicit profile makes the profile set isolated instead:

```console
sm shell --profile common --profile matt
```

Add individual skills with profile-qualified references:

```console
sm shell --skill research/web-search
```

Without an explicit profile this adds to the global set. With explicit profiles it adds to that isolated set. Selectors are resolved left to right; later selectors win temporary duplicate names.

## Child Shell Behavior

The child inherits the current working directory and normal environment. Arguments after `--` are passed to the child shell:

```console
sm shell --target pi -- -c 'pi --model sonnet'
```

sm places generation wrappers first in `PATH` and sets:

```text
SM_SKILLS_DIR=<generation>/skills
SM_GENERATION=<generation-id>
```

Only configured command basenames are wrapped. Other commands behave normally. Agent commands without a shell adapter continue to use their ordinary global behavior.

## Startup Files

- **zsh:** generated startup shims source the user's real zsh files and restore wrapper precedence.
- **Bash:** a generated `--rcfile` and `BASH_ENV` source user startup state and restore wrapper precedence. Login shells are rejected because they bypass `--rcfile`.
- **Fish:** `--init-command` restores wrapper precedence after `config.fish`.
- **Other shells:** receive wrapper-first `PATH`; their startup files must preserve it.

## Generations and Leases

A generation is stored under the cache:

```text
~/.cache/sm/generations/<id>/
  skills/
    code-review -> ~/.sm/profiles/matt/code-review
  bin/
    pi
    other-agent
```

Its membership and wrapper set never change in place. Multiple child shells can therefore use different generations concurrently. Live child shells and wrapped commands hold PID leases.

Remove unleased generations with:

```console
sm gc
```
