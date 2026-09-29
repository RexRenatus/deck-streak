---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The settings screen writes stored runtime settings only, each through its owning context's setter, and configuration read at start stays configuration

## Context and Problem Statement

#57 asks for one Mini App screen for every runtime setting, and earlier waves deferred their hidden
settings to it. DeckStreak holds two kinds of value that a reader could call a setting: stored
runtime settings (rows a context reads at each use, such as the quiet window or the chest cap), and
configuration read once at start (the environment's caps and switches, the owner's private files,
credentials). Each stored setting lives in the table of the context that owns it (the CONTEXT-MAP's
ownership register). Which of them may the screen write, and through what?

## Decision Drivers

- A context's table is written only by that context (the CONTEXT-MAP's ownership rule).
- A value read at start takes effect only after a restart, so a screen that wrote it would show a
  value the running service does not use.
- A credential is a secret: it is never shown on a device (SPEC-066).
- The settings generation must move in the same write as the value, so every reader sees the change
  (SPEC-020's settings rule).
- Export and erase stay symmetric: a new setting's default is known and restorable (SPEC-021).

## Considered Options (the alternatives it was chosen against)

- Each owning context declares its settings beside its setter: chosen, because the owner of each
  table stays its only writer, and coordination's census gathers and routes each write, so one
  screen still reaches every setting.
- One generic key-value settings table that the API writes directly: rejected because the API
  would write rows that belong to other contexts, which the ownership register forbids.
- Make start-time configuration editable from the screen: rejected because a change would apply
  only after a restart, and a credential would be shown on a device.
- Keep settings on the device (Telegram's cloud or device storage): rejected because the server's
  jobs read the settings, and a device copy would drift from the value the jobs use.
- A bot command per setting: rejected because each command would repeat a parser that the screen's
  typed controls make unnecessary, and #57 asks for a screen.

## Decision Outcome

Chosen option: "each owning context declares its settings beside its setter, and the census routes
each write", because it is the only option that keeps each table's single writer and still gives
the owner one screen.

- **The declaration.** A `SettingDecl` in the kernel (key, section, shape, default); a key declared
  twice refuses the census at start by name.
- **The write.** The census routes to the owning setter, which checks the value against its shape
  and writes it with the settings generation in one `BEGIN IMMEDIATE` transaction; an equal value
  writes nothing. The census names the refusals `unknown_setting` and `value_invalid`.
- **The defaults.** `settings.defaults.json` holds every default, and no sharing or public switch is
  on by default.
- **What stays configuration.** Everything read at start, named with its reason in SPEC-130 §2a.

### Consequences

- Good, because a later SPEC adds its setting by declaring it, and the screen shows it with no
  change to the census.
- Good, because the owner sees on the screen the value the route stored, never an optimistic one.
- Bad, because a setting that needs a restart cannot be changed from the screen; the screen names
  none of them, and the owner changes them through the deploy.

### Confirmation

SPEC-130 §3 (the census, its refusals and the screen's criteria) and its rows in `S13000-S13099`.

## What would make this wrong

- A setting that must change without a restart but is read only at start: it would move into a
  context's table and gain a declaration, rather than make configuration writable.
- A second owner or a shared screen: the owner-only gate (ADR-024) would need its own decision.

## More Information

SPEC-130, SPEC-020, SPEC-021, SPEC-024, SPEC-041, ADR-024, ADR-041, ADR-087, ADR-096, and the
CONTEXT-MAP's ownership register.
