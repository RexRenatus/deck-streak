---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The one-router census follows what the compiler pulls in, refuses what it cannot follow, and names the rest

## Context and Problem Statement

SPEC-041's A15 census (`crates/notifications/tests/one_router.rs`) reads the tree as text: every
shipped source of the kinds it names, by its path, with the rules that path owes. SPEC-041 §5 and
ADR-041's last Bad bullet list what it leaves unread (#297). The issue's six bullets hold nine
classes once the three joined bullets are split, and each is decided here: closed by a rule the
census can hold, or kept and named with the reason a text census cannot hold it.

Measured at dev `02758d4`:

- 24 includes in shipped Rust, all `include_str!`. One, in `crates/daemon/src/wiring.rs`, sits in a
  `#[cfg(test)]` module. The other 23 bring in the 18 persona files (`crates/agent/src/persona.rs`),
  `notifications-policy.json`, `economy.json` and three files under `crates/vault/data/`. There is
  no `include!` and no `include_bytes!`.
- 25 migrations under `migrations/`, applied at every start (`sqlx::migrate!`,
  `crates/kernel/src/db.rs:34`). The feed's and the held queue's tables are named in the router's
  two migrations only, and no migration names the Bot API.
- No tracked symlink.
- Two `#[path]` attributes in shipped sources, both on `#[cfg(test)]` modules.
- The release copies `deploy/` and `agent/` whole (`.github/workflows/release.yml:89-90`), and
  `agent/tests/` holds three files, so a unit could run a test file the census does not read.
- The command handler's 17 replies and dispatch (`COMMAND_REPLIES`) are each defined once, as a
  private method of its inherent impl (`crates/bot/src/commands.rs:441`).

## Decision Drivers

- The census guards ordinary code; it is not a sandbox against code written to evade it, which
  review catches (SPEC-041 §5).
- A literal the census can read is no reason to leave a file unread: an include, a `#[path]`, a
  migration and a unit's path are each spelled as one.
- A name the census cannot resolve is refused, never guessed (SPEC-041 A16, A17).
- Every rule is proved on a planted case and on the committed tree, which must read no refusal.

## Considered Options (the alternatives it was chosen against)

- D1, follow each `include!`, `include_str!` and `include_bytes!` whose argument is one string
  literal, read the file under its own path (as Rust from `include!`, as text otherwise) and refuse
  one the census cannot name or find: chosen, because the 23 includes at dev are literals and none
  names a held name (#297).
- D1, keep includes named: rejected, because the file the compiler includes is a literal the census
  can read, so naming it leaves ordinary code unread (#297).
- D1, refuse every include outright: rejected, because it would red on 23 ordinary includes, the
  policy file and the persona files among them (#297).
- D2, SQL joins the shipped kinds, and only the router's two migrations may name the feed or the
  held queue: chosen, because a migration runs at every start, so a write to the queue in one is a
  delivery the flush makes (#297).
- D2, read every file of every kind: rejected, because 1227 files at dev are of kinds the census
  does not read, 639 Markdown and 335 JSON among them, and the prose under `docs/` names
  `sendMessage` throughout (#297).
- D3, the walker refuses each symlink it meets: chosen, because a symlink is neither read nor
  followed, so a delivery behind one would go unread, and the tree holds none (#297).
- D3, follow symlinks: rejected, because a link can leave the tree or loop, and what it points at
  is read under a path that is not its own (#297).
- D4, follow a module's `#[path = "…"]` outside the notifications crate, read its target as Rust
  under its own path, and refuse a target not in the tree or a `#[path]` inside a block: chosen,
  because a test file a shipped crate compiles is shipped code (#297).
- D5, refuse a unit or drop-in whose path runs a test file, by a test directory with no `src`
  before it or by a test file's name: chosen, because the release ships `agent/tests/` and the
  census reads no test file (#297).
- D6, refuse a reply of the command handler defined `pub`, `pub(...)` or in a trait impl for the
  handler: chosen, because with every reply private the compiler refuses a call from outside the
  handler's module, and A15's named callers already hold a call inside it (#297).
- D6, a census of the replies' calls across the bot crate: rejected, because it needs name
  resolution through `use … as` and traits, which the compiler's privacy already does (#297).
- Keeping classes 1, 2b's other kinds, 5 and 6a, a regex over constant initializers or wrapper
  bodies: rejected, because one `use … as` inside a block defeats it, so the census would claim to
  resolve names it cannot (#297).
- Keeping class 6a, a census of the router modules' 95 `pub` lines: rejected, because it holds
  names, not writes, and adds friction to every router delivery while the queue's writes are
  already `pub(crate)` (#297).

## Decision Outcome

Chosen: D1 to D6 close classes 2a, 2b for SQL, 3, 4a, 4b and 6b, each in the census and each by
its criterion (SPEC-041 §12, A24 to A29). Classes 1, 2b's other kinds, 5 and 6a stay unread and
are named with the reason in SPEC-041 §11, and so is D5's residual: a test file that a shipped
script runs (`scripts/check.sh` runs `agent/tests/`), or a path a unit assembles from a specifier
or an environment variable. No production code changes; every rule is test code.

### Consequences

- Good, because the files the compiler, the migrator and the units pull in are read under their
  own paths with every rule those paths owe, and A15 prints how many it examined.
- Good, because what still goes unread is named with a reason a reviewer can check.
- Bad, because a local untracked symlink in a walked directory reds A15 by its path until it is
  removed.
- Bad, because a build script that generates included code is refused, not read: its file is
  named by `concat!(env!("OUT_DIR"), …)`, which the census cannot name.

### Confirmation

SPEC-041's A15 and A24 to A29, and the rows S04182 to S04193.

## What would make this wrong

- A build script that generates included code: the census would then have to read the generated
  file, which only a build can name, and G1's refusal would have to give way to a reading of the
  build's output.
- A delivery that needs a symlink in the shipped tree: the walker's refusal would then need a rule
  for what the link may point at.
- A reply of the command handler that must be visible outside its module: the handler would then
  need a named caller outside the module, and the visibility rule an admitted name.

## More Information

Issue #297; SPEC-041 §5, §11 and §12; ADR-041's last Bad bullet and its amendment;
`docs/schematics/notification-router.md`, "The census's reach (#297)".
