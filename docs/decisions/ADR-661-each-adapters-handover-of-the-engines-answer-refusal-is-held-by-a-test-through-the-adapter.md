---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-661: each adapter's handover of the engine's answer refusal is held by a test through the adapter

Status: accepted. Decides SPEC-638 (issue #937). Follows ADR-477, which pins the refusal at the
core's answer door (SPEC-460, #760), and changes none of its decisions.

## Context

The engine refuses an answer whose current state is no longer the card's (`card was modified`, an
invalid-input `BackendError`), and the core hands that refusal back from its answer door as
`AnswerRefusal::Engine { error }`. SPEC-460's tests hold both at the core. Between the core and the
owner sit two adapters, each with its own mapping of that refusal, and no test runs either:

- native: `Engine::answer` (`crates/ffi/src/engine.rs:203-232`) returns the core's refusal through
  `press_refusal` (`:226`), whose arm at `:739` hands the engine's bytes on as
  `PressRefusal::Engine { error }`;
- web: `rate` (`crates/web-engine/src/wasm.rs:874-912`) throws the core's refusal to the Worker by
  `.map_err(refuse)?` (`:892`), before it forgets the shown card (`:893`), and the Worker codes the
  thrown sentence as `engine-failed`.

Either mapping could swallow the refusal, so that a refused answer reads as recorded, or name the
wrong reason, and nothing would turn red. The schematic's section 6 table says so for both: "the
adapter's own lines are not run here". Every `path:line` here was read at `dev` `36efa139`.

## Decision

### D1. Each handover is pinned through the adapter's own public entry

Each test drives the entry its caller uses: the native `Engine::answer`
(`crates/ffi/src/engine.rs:203-232`), as a native client calls it, and the web `rate` reached
through the Worker client (`web/app/src/lib/engine/client.ts`' `rate`), as the page calls it.
Neither test calls the core's door directly: SPEC-460 already holds that, and a test there would
not run the adapter's mapping.

### D2. The native stale card is made by a second press

The native test presses Good on the states the queue showed, then presses again on the same
states: the first press changes the card, so the second is stale. This is SPEC-460 A1's route,
taken through the adapter. It needs no exempt tap, so the containment census
(`crates/engine-core/tests/containment.rs`) gains no held line, and the test names none of the
engine's or the gesture's names.

### D3. The web stale card is made by a sync

Within one Worker, `rate`, `undo` and `bury` each forget the shown card (`wasm.rs:893`, `:431` and
`:943`), and a flag leaves the card's scheduling state equal (ADR-477 D1), so none of them can make
the shown card stale. `sync_collection` (`wasm.rs:691-705`) keeps the shown card. So a second
browser's answer of the same card, synced into the first while it still shows the card, is the one
route the Worker client offers by which the shown card's state changes under the Worker's kept
states. (The module's exported `run_exempt`, `wasm.rs:548-563`, also leaves the shown card set, but
`client.ts` exposes no exempt method.) The test is built from the browser test file's own helpers,
two browsers in their own profiles, as its test `an upload waits for its snapshot` does.

### D4. The web handover is also held as text

The web module runs only in a browser, so no mutation row's killer can run the browser test. The
web boundary census (`crates/web-engine/tests/boundary.rs`) therefore owes `rate` the refusal's
statement, written as `&request.encode_to_vec()).map_err(refuse)?`: compared with every blank
removed, it holds the mapping, and the literal names no engine or gesture name, so the containment
census, which also reads string literals, stays green unchanged.

### D5. Not red first

Both handovers are on `dev`, so no criterion can be red at the base for its reason without
changing what is pinned. Each criterion's red is its rows: A1's are S63801 (the refusal named as
another card) and S63802 (the refusal swallowed), A3's is S63803 (the web refusal swallowed). A2
has no row and is `not red`; its text is held by A3's row.

### D6. Formal not required

No state, protocol or concurrency is added. `formal/tla/AnswerOnTheShownState` covers the core's
door and is unchanged, and no file it covers changes.

## Consequences

- Good: a mapping that swallows the engine's refusal, or names the wrong reason, now fails a test
  through the adapter that holds it: A1 natively, A3 as text and A2 in both browsers.
- Good: the schematic gains a section that states each handover at the current `dev` and names
  the test and the rows that run it.
- Bad: the browser test takes two browsers and three syncs, so it is among the slower tests of
  its file.
- Bad: an engine bump that rewords the message head turns A1 red; the bump's delivery re-reads the
  head from the engine's source and re-pins it.
- Neutral: the census's new statement holds the web mapping's text, not its behaviour; the
  browser test holds the behaviour.

## Rejected options

- A test-only export of the web module: rejected, because it widens the boundary the census holds,
  and a test would then run an export no page calls.
- An exempt due-date tap in the native test: rejected, because it adds a held line to the
  containment census and reaches no line the second press does not.
- A host-side test of the web mapping: rejected, because the web module has no native engine:
  `crates/web-engine/Cargo.toml` puts the engine core behind `cfg(target_arch = "wasm32")`.

## What would make this wrong

- If the Worker client gains an exempt method, D3's route is no longer the only one, and the
  browser test may take the shorter one.
- If the engine stops comparing the current state, SPEC-460's tests turn red first, and these tests
  with them.
