# Skill Management

This context describes how skill collections are owned, made globally visible, and temporarily scoped for agent sessions.

## Language

**Skill**:
A named, self-contained capability directory managed as an opaque unit.

**Profile**:
A named collection of skills whose existence and properties belong to one profile directory.
_Avoid_: Group, package

**Global Activation**:
The profile property that determines whether its skills belong to the normal visible skill set. Activation is shared by every target.
_Avoid_: Target activation, enabled marker

**Import**:
An inventory operation that adds skills to a profile and may explicitly create that profile. Imported source directories remain owned by their source.

**Update**:
An inventory operation that replaces existing managed skills from same-named source directories. It never creates profiles or skills.

**Apply**:
A projection operation that reconciles every target with the globally active skill set.
_Avoid_: Sync

**Adopt**:
A configuration operation that registers a skill directory as a target.
_Avoid_: Import, move

**Target**:
A configured destination where the globally active skill set is exposed. A target does not select profiles or own activation state.
_Avoid_: Profile scope

**Adoption**:
An ownership transfer that moves a skill directory into a profile and leaves managed visibility in its place when globally active.
_Avoid_: Copy, synchronization

**Shell Adapter**:
A target capability that starts an agent command with an explicitly supplied skill set.
_Avoid_: Shell target

**Isolated Shell**:
A child command shell in which configured agent commands use a session-specific skill set while the global skill set remains unchanged.
_Avoid_: Target shell

**Generation**:
A stable, session-specific resolved skill set used by one or more shell adapters.
_Avoid_: Enabled state
