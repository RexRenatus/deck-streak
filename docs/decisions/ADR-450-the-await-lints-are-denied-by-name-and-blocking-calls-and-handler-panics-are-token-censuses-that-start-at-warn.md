---
status: "accepted"
---

# ADR-450: the await lints are denied by name, and blocking calls in async bodies and panics in the request-handler crates are token censuses that start at warn

Decides SPEC-436. Path:
`docs/decisions/ADR-450-the-await-lints-are-denied-by-name-and-blocking-calls-and-handler-panics-are-token-censuses-that-start-at-warn.md`.
Amends nothing. Cites ADR-293 (the source pins read Rust as tokens), SPEC-002 A2 (every crate
inherits the workspace lint table), SPEC-020 R14 (the offload is the rail blocking work takes),
SPEC-025 (the API's panic catcher) and SPEC-041 A18 (the audit that no lint group holding a
denied lint is silenced). Read at dev `b25ed04a`.

## Context

Three robustness rules for the server's async code, each a defect class: a lock guard held across
an `.await` (a task parks while a thread-blocking lock is held, so every other task that wants it
stalls the worker it runs on); a blocking call inside an async body (a runtime worker does file,
network or process work, and every task queued on it waits); and a panic in a request-handler
crate (a request crashes its task instead of being answered with an error).

What the tree holds at dev `b25ed04a`:

- **The clippy stage denies every warning.** `scripts/check.sh:85-88` runs
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, and CI's `rust` job runs that
  stage (`.github/workflows/ci.yml:67-70`). `[workspace.lints.clippy]` (`Cargo.toml:182-190`) sets
  the `all` and `pedantic` groups to warn at priority -1, and `unwrap_used`, `expect_used`,
  `dbg_macro` and `print_stdout` to warn. All 31 members inherit it (`[lints] workspace = true`,
  held by SPEC-002 A2). `clippy.toml:2-3` allows unwrap and expect in tests.
- **So the await lints already fail the stage, but only through a group and a flag.**
  `await_holding_lock` and `await_holding_refcell_ref` are in clippy's `suspicious` group, which
  `all` holds. Nothing names them, and nothing audits a lowering of either one or of `suspicious`:
  SPEC-041 A18's audit refuses a silenced `all`, `style` or `warnings`, for its own two lints.
- **No lock guard is held across an await today.** The shipped sources take `std::sync::Mutex`
  and `RwLock` guards in api, coordination, daemon, engine-core, ffi, identity, ingest, kernel, mcp,
  notifications and push; no crate uses an async-aware or a `parking_lot` lock. The clippy stage
  reports no guard held across an await, which this delivery's own CI run re-derives, and the
  guards read in async bodies are temporaries or block-scoped bindings:
  `crates/api/src/snapshot_routes.rs:120` and `:129` read and write the cache as statement
  temporaries around `lister.list().await`, and `crates/daemon/src/sync_request.rs:157-166` holds
  `last_ring` inside a block that ends before `pause(..).await`.
- **No clippy lint is scoped to async bodies for blocking calls.** `disallowed-methods` in
  `clippy.toml` is crate-wide, so naming `std::fs::read` there would refuse the vault's and the
  kernel's synchronous file code, which is correct where it runs. The house rail for blocking work
  is the kernel's offload (`crates/kernel/src/offload.rs:53-80`, SPEC-020 R14), beside tokio's
  `spawn_blocking`.
- **Eleven blocking calls sit in async bodies, in three files.** `crates/agent/src/gate.rs:205`,
  `:206` and `:220` remove a staged file with `std::fs::remove_file` inside the `async move` blocks
  of `check` and `check_input`; `crates/agent/src/runner.rs:91` does the same in `run`'s block.
  `crates/ingest/src/skip_write.rs` copies the collection with `std::fs::copy` in `steps` (`:302`)
  and in `backup` (`:511`), sets its mode and renames it there (`:512`, `:519`), and reads two whole
  collections and writes a third in `restore_check` (`:795`, twice, and `:804`). Every other
  `std::fs`, `std::thread::sleep`, `std::process::Command` or `block_on` call in the shipped sources
  is in a synchronous function or inside a `spawn_blocking` or offload closure (for example
  `crates/ingest/src/lock.rs:55` and `:85`, `crates/agent/src/gate.rs:154`,
  `crates/daemon/src/role_data.rs:104`), and the agent's commands are tokio's
  (`crates/agent/src/gate.rs:14`, `crates/agent/src/runner.rs:16`).
- **The request-handler crates hold no panic path in shipped code.** `api` holds the routes,
  `bot` the commands and callbacks, `mcp` the tools behind the bearer guard, and `daemon` the roles
  that serve them and the adapters their ports reach (`crates/daemon/src/lib.rs:3-14`). The API and
  MCP routers answer a panic with a 500 (`crates/api/src/router.rs:319`,
  `crates/mcp/src/server.rs:124`; SPEC-025); the bot's long poll has no catcher. In the four
  crates' shipped sources there is no `panic!`, `unreachable!`, `todo!`, `unimplemented!`,
  `assert!`, `assert_eq!`, `assert_ne!`, `.unwrap()` or `.expect(`: every one of them sits in a
  `#[cfg(test)]` module (`crates/daemon/src/wiring.rs:816-1783`, `crates/mcp/src/grants.rs:218`,
  `crates/daemon/src/lifecycle.rs:278`, `crates/daemon/src/main.rs:150`,
  `crates/daemon/src/role_job.rs:295`).
- **The tokens reader is admitted.** ADR-293 reads Rust through `proc-macro2`'s lexer, already in
  `Cargo.lock` and a workspace dependency with no default feature (`Cargo.toml:74`), so comments
  and string literals are never code.

## Decision Drivers

- No rule already in force is lowered. A lowering is a weakening, which needs the owner's signed
  ruling.
- A new rule starts at warn and is promoted to fail after three iterations of the rule review at
  precision of at least 0.8 with at least five hits; demoting a fail rule is a signed ruling.
- Every check prints what it examined and fails on an empty population, which is VOID, never a
  pass.
- Code is read as the compiler reads it: a name in a comment or a string is not a call.
- No product code changes in this delivery: today's sites are listed, not cured.

## Decisions

**D1. The await lints are named at deny (SPEC-436 R1-R4).** `[workspace.lints.clippy]` gains
`await_holding_lock = "deny"` and `await_holding_refcell_ref = "deny"` at the default priority,
above the groups' -1, so they hold whatever `all` or `suspicious` is set to and without the
stage's warnings-denied flag. Clippy is the matcher: it reads the types a coroutine keeps across
each suspension point, so a guard taken through a helper (`crates/identity/src/linking.rs:143`'s
`lock`) is seen by its type, whatever binds it. Its population is every target of every member,
tests included, under the stage's `--workspace --all-targets`; at dev it reports 0 in every crate,
which this delivery's own CI run re-derives. It lands at deny, not at warn: the rule is in force
today through `all` and the stage's flag, so a warn start would lower it. A test audits that no
attribute, member manifest or compiler flag lowers either lint or `suspicious`; a suppression is
admitted only as a listed `#[expect(clippy::<lint>, reason = "...")]`, and the listing is empty
at dev. Known classes:
- **False positive:** a guard dropped by an explicit `drop(guard)` before the await is still
  reported (clippy's documented known problem); the cure is a block scope, never a suppression.
- **Not a hit, correctly:** an async-aware mutex (tokio's) may be held across an await, and a guard
  dropped by its scope before the await is not reported.
- **False negatives:** a lock type outside `std::sync` and `parking_lot` (none at dev; clippy's
  `await-holding-invalid-types` names one when it arrives); a guard whose span comes from another
  crate's macro, which clippy may leave unreported; a crate that stops inheriting the table (held by
  SPEC-002 A2); `all` or `warnings` silenced (held by SPEC-041 A18).

**D2. Blocking calls in async bodies are a token census, measured first (SPEC-436 R5-R12).** The
census reads every member's shipped sources (each `crates/<member>/src/**/*.rs`, less test code:
items under `#[cfg(test)]` or `#[test]`, and the out-of-line and `#[path]` test modules they
declare) as `proc-macro2` tokens. An async body is an `async fn`'s body, an `async` or
`async move` block, or an async closure's body. Inside one, a call is a blocking site when its path,
resolved through the file's `use` declarations (a group, `self` and an `as` rename included),
names a member of the blocking set, each with its async equivalent:
- `std::fs::{read, read_to_string, read_dir, write, copy, rename, remove_file, remove_dir,
  remove_dir_all, create_dir, create_dir_all, metadata, symlink_metadata, set_permissions,
  canonicalize}`, `std::fs::File::{open, create, create_new}` and `std::fs::OpenOptions::new`:
  `tokio::fs`, or the offload for a whole-file or engine operation;
- `std::thread::sleep`: `tokio::time::sleep`;
- `std::net::TcpStream::connect`, `std::net::TcpListener::bind` and `std::net::UdpSocket::bind`:
  `tokio::net`;
- `std::process::Command::new`: `tokio::process::Command`;
- `std::io::stdin`: `tokio::io::stdin`;
- any path under `reqwest::blocking`: the async client;
- a `block_on` call, by path or as a method: `.await`;
- `.recv()` in a file that imports `std::sync::mpsc`: tokio's `mpsc`, or `spawn_blocking`.

Not an async body: the closure argument of a rail, `spawn_blocking`, `block_in_place` or a `run`
method called on a receiver named `offload` (SPEC-020 R14), with everything inside it; a nested
`fn` item; test code. A path named and not called (`File::lock` handed to a helper,
`crates/ingest/src/lock.rs:43`) is not a call. A site's key is its file, its enclosing function's
name, its resolved path and its ordinal among that function's calls of that path, never a line
number. Its population at dev is 338 shipped files (340 under `crates/*/src`, less
`crates/ingest/src/skip_write/tests.rs` and `crates/vault/src/unicode_other_tests.rs`), and its
sites are D5's eleven. It lands at warn. Known classes:
- **False positives:** a closure handed to a crate's own helper that runs it on the blocking pool
  (`crates/ingest/src/skip_write.rs`'s `on_pool` and `call_hook`) is examined as async; a call in
  an async body that runs before the runtime serves; a call whose block is short (a small file's
  unlink) is still a site.
- **False negatives:** a synchronous helper called from an async body
  (`crates/agent/src/runner.rs:89`'s `write_private`, `crates/ingest/src/skip_write.rs`'s
  `remove_older_backups` and `remove_with_journal`, the daemon's doorbell file write); a blocking
  method on a value (`sync_all`, `Child::wait`, `JoinHandle::join`, a `recv` without the import);
  a member outside the set (`hard_link`, `read_link`, `exists`, `connect_timeout`, `File::options`,
  `recv_timeout`, another crate's blocking client, CPU-heavy work such as hashing a whole file); a
  glob import or a re-export from another module.

**D3. Panics in the request-handler crates are a token census (SPEC-436 R13-R17).** The
request-handler crates are `api`, `bot`, `mcp` and `daemon`, named by the census, not by a
pattern. Its population is their shipped sources, read as D2 reads them: 55 files at dev
(22, 14, 7 and 12). A site is a `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `assert!`,
`assert_eq!` or `assert_ne!` invocation, bare or `std::`- or `core::`-qualified, or a method call
`.unwrap()`, `.expect(..)`, `.unwrap_err()` or `.expect_err(..)`. Not a site: `debug_assert!` and
its forms, which a release build removes; the body of a `const` item and a `const { }` block,
evaluated at compile time; test code; `unwrap_or`, `unwrap_or_else` and `unwrap_or_default`. Its
sites at dev are 0. It lands at warn. Clippy's `unwrap_used` and `expect_used` stay at the
workspace's warn under the stage's flag, which already fails an unwrap or expect in every crate;
the census adds the macros and the `#[expect]` escape for these four crates, and lowers nothing.
Errors map to responses: a handler returns `Result<_, E>` where `E` answers with a status, as the
routes do today. Known classes:
- **False positives:** an infallible conversion's `unwrap` or `expect`; start-up code before the
  service answers; an `expect` method of a type that is not `Option` or `Result`.
- **False negatives:** indexing and slicing; arithmetic overflow in a debug build; division by
  zero; a `RefCell` borrow; a panic in a crate the handlers call (the economy, progression and xp
  crates allow an `expect` on their embedded file, `crates/xp/src/table.rs:50-53`); a macro from a
  dependency that panics.

**D4. Three checks, three homes, one shared reader (SPEC-436 R18-R20).** D1's audit is
`crates/daemon/tests/await_lints.rs`; D2's census is `crates/daemon/tests/blocking_census.rs`; D3's
is `crates/daemon/tests/handler_panic_census.rs`. D2 and D3 share the reader
`crates/daemon/tests/shipped_sources/mod.rs` (the walk, test-code exclusion, tokens, `use`
resolution, async bodies and the level's verdict), each binary including it by `mod`. The daemon is the
composition root that runs every role's runtime, and `crates/daemon/tests/workspace.rs` already
holds the workspace's shape there. The daemon takes `proc-macro2.workspace = true` as a
dev-dependency, citing ADR-293, so the lock gains one dependency edge and no crate. Each check
prints `examined <n> <unit>` and fails on 0 as VOID. All three are Rust tests read locally by
cargo, so no module enters `scripts/tests/test_ci_workflows.py`'s `DYNAMIC_IMPORTS`. Each census
function takes its sources as data, so every planted control is in-memory text and no test writes
a file.

**D5. The listings at dev, and the levels (SPEC-436 R21-R24).** Each census carries a `LEVEL`
constant and a `LISTED` table. At `Level::Warn` an unlisted site prints `WARN <rule> <key>` and the
test passes; at `Level::Fail` it fails, naming the site. At both levels a listed entry that matches
no site fails, so a fix removes its entry, which is how a fix is counted. D2 lists the eleven sites
of the Context section; D3 lists none; D1's suppression listing is empty. A listed site that looks
like a defect is listed, not cured, here: curing is a change of product code, and two classes of it
(the skip day's whole-file work and the agent's unlink) are tracked as their own work (SPEC-436
section 5).

**D6. Kill criteria, for each check.** For D2 and D3, the promotion rule of the Decision Drivers:
`LEVEL` moves from warn to fail after three iterations of the rule review at precision of at least
0.8 with at least five hits, by its own delivery; demoting a fail rule is a signed ruling. For all
three checks:
- **Live precision** is fixes ÷ (fixes + new suppressions or listing entries) over the last 30
  merged PRs. Below 0.8 with at least 5 hits, propose a demotion.
- **New zero-hit rules** (D1 and D3): if 2 or more of the rule's first 5 hits are false, propose a
  demotion.
- **Demotion** is a weakening, and goes to the owner as a signed ruling.
- **Tooling:** `examined 0`, or a tool that is missing or off its pin (clippy off the toolchain
  pin, the tokens reader absent), is VOID, which blocks. It is never a pass.

**D7. Not in this delivery.** No other lint rule, no clippy sweep of other zero-hit lints, no cure
of a listed site, no CI or test-runner configuration change: SPEC-436 section 5 names each with
its issue.

**D8. Tests and rows (SPEC-436 sections 3 and 9).** One test per acceptance criterion, red first:
the red commit holds the tests and stubs that keep every input but the missing behaviour (the
readers find nothing, the verdict passes everything, the refusals refuse nothing), so each red is an
assertion on the missing behaviour over a planted fixture or the repository's own positive
artifacts, never a wall-clock reading. Every red of a planted-text test is read locally by cargo (A4's, A8's and A13's, which read the repository's own sources, are read by name in CI at push 1), so the red commit is
pushed alone first for size safety, and the greens and the record follow in a second push. Rows
S43600 to S43690 in `scripts/mutation-rows.d/S43600-S43699.json`, hand-written over the three
tests and the reader, each killed by a test of its own binary.

**D9. One delivery, no amendment.** SPEC-436 and this ADR carry the three rules. SPEC-002,
SPEC-020 and SPEC-041 state nothing this delivery changes, so none is amended. A new schematic,
`docs/schematics/async-robustness-checks.md`, draws the three checks.

## What it was chosen against

### D1

- A hand census of guards with no clippy lint: rejected because text cannot see a guard's type or its lifetime across a suspension point, which clippy reads from the coroutine itself.
- Starting the await lints at warn: rejected because they already fail the stage through `all`, so a warn start lowers a rule in force without a signed ruling.
- `forbid` in place of `deny`: rejected because it refuses even a reasoned, listed `#[expect]`, leaving no recorded escape for the documented false positive.
- Relying on the group and the flag alone: rejected because one `#![allow(clippy::suspicious)]` silences both lints unseen.

### D2

- A runtime detector (a hook timing each poll, or tokio's unstable task instrumentation): rejected because it reports only what a run happens to execute, needs a timing threshold a loaded runner would make flaky, and names no source site.
- Clippy's `disallowed-methods`: rejected because it is crate-wide and cannot tell an async body from the synchronous file code that is correct.
- A text regex over sources: rejected because it reads a call in a comment or a string as code, the class ADR-293 closed with tokens.
- A full parse with `syn`'s item visitor: rejected because it needs features the workspace does not enable and a token walk already finds async bodies, rails and calls.

### D3

- A `[workspace.lints]` rule over every crate (`clippy::panic` and its siblings): rejected because the domain crates hold reasoned start-up panics on embedded files, and a workspace rule would lower nothing yet refuse them all at once.
- Per-crate clippy restriction lints: rejected because every member inherits the workspace table, which a member's own `[lints]` cannot extend, so it needs crate-root attributes in four product files, and clippy at warn already fails the stage, which leaves no warn start.
- Including indexing and slicing: rejected because a token census cannot tell an index that can panic from one that cannot; it stays a false-negative class with its own issue.

### D4

- One combined module for all three checks: rejected because one verdict would hide which rule failed, and a level change for one rule would move the others' evidence.
- Python under `scripts/tests/`: rejected because a loader-style module there is read only in CI, while cargo reads Rust tests locally, and ADR-293 already admitted the Rust tokens reader.
- The kernel or the agent crate as the home: rejected because the panic census names four crates, which the kernel may not know, and the agent is a context, not the runtime's owner.

### D5

- Curing the listed sites in this delivery: rejected because each cure is a product change with its own tests and, for the skip day's `restore_check`, its own formal re-read.
- Keying sites by line number: rejected because any edit above a site would break its entry.

### D6

- A kill window in days: rejected because the window is the last 30 merged PRs, which a test or a reader can recount.

### D8

- One push: rejected because a size stop would leave the red commit only in a local clone.

### D9

- Amending SPEC-002 to name the await lints: rejected because SPEC-002 states the table's inheritance, not its entries, and SPEC-436 holds the entries.

## Consequences

- Good: the await lints hold by name, whatever a group or a flag says, and a lowering anywhere is
  refused by name.
- Good: every async body in the workspace and every shipped line of the four handler crates is
  examined on each run, and a new site is printed with its key.
- Good: a fix is counted when its listing entry is removed, so live precision is read from merged
  pull requests.
- Bad: at warn a new blocking site or handler panic passes; it is visible in the test's output, and
  CI keeps a passing test's output only in a run that asks for it (SPEC-436 section 5).
- Bad: the census's false-negative classes (synchronous helpers, value methods, indexing) stay
  open until their issues land.
- Bad: the daemon's test build gains a dependency edge, already compiled for the workspace's macros.

## Confirmation

SPEC-436's acceptance criteria A1 to A14, each read red then green, recorded in
`docs/red-first/SPEC-436.md`. The rows S43600 to S43690. The clippy stage of the delivery's own CI
run, green with the two lints at deny.

## What would make this wrong

- A guard that clippy cannot see is held across an await in the tree. Then D1's matcher is
  incomplete, and a census for that lock type is a new decision.
- The census lists a site the review finds correct in more than 1 of 5 cases. Then its precision
  falls, and D6 proposes a demotion to the owner.
- A blocking call reaches an async body only through helpers, so the census holds 0 while the
  runtime blocks. Then its false-negative issue becomes the next rule.
- The tokens reader refuses a shipped file that rustc accepts. Then the census fails by name on
  that file, closed, never a skip.

## More Information

SPEC-436 (requirements, criteria, manifest); ADR-293 (Rust read as tokens); SPEC-002 (the
workspace lint table's inheritance); SPEC-020 R14 (the offload); SPEC-025 (the API's panic
catcher); SPEC-041 A18 (the audit of silenced lint groups). Issue #822 is the release; this
delivery lands after it.
