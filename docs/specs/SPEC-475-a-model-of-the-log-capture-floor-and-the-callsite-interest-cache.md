# SPEC-475: a model of the test log capture's floor against the callsite interest cache

- **Status:** in delivery: the model, its configuration and its three witnesses, decided by
  ADR-492; it closes #499, which follows #461.
- **Decided by:** ADR-492 (D1 the mechanism, D2 the model, D3 no cure, D4 one delivery, D5 the
  criteria and the covers, D6 the exclusions), and SPEC-024 section 8, whose class this model checks.
- **Path:** `docs/specs/SPEC-475-a-model-of-the-log-capture-floor-and-the-callsite-interest-cache.md`

## 1. The problem, measured

Every figure was read at dev `36c4783a` by `git show <dev>:<path>` with `grep -n`, by `git grep` and
`git ls-tree` over the same commit, and, for the locked `tracing-core`, from its own `src/` files.

- **The capture is held by chosen orders.** `crates/kernel/tests/log_capture_class.rs:260` runs four
  scenarios (`SCENARIOS`), each in a child process: a scoped and a held capture, each with another
  thread reaching the line inside the capture, and each with another thread's registration held
  between its rebuilder's choice and its answer (`:152-155`, `:224-225`). Its doc says an
  answer asked with no default and stored after a capture's is "an order no scenario here needs to
  reach" (`:19-21`). No test explores every order.
- **No model covers the helper.** `formal/tla/` holds 27 entries (`git ls-tree --name-only <dev>
  formal/tla/`), and the 2 `@phx covers` lines that name a file with `capture` in its name name
  vault files; 0 name `tools/log-capture/capture.rs`.
- **The helper's pins.** The census asserts 25 routed captures
  (`crates/kernel/tests/log_capture_class.rs:1588`), and 4 mutation rows anchor on the helper
  (S02407 to S02410, `scripts/mutation-rows.d/S02400-S02499.json:60-95`).
- **The mechanism the floor relies on**, each step cited in ADR-492 D1: the level filter starts at
  `OFF` and only a full rebuild sets it; a callsite's first registration asks only its own thread's
  default, with no lock, while at most one dispatcher was registered at the last registration;
  the answer is stored without a lock and can land after another thread's; and the floor hints
  `OFF` (`tools/log-capture/capture.rs:57-59`), answers "sometimes" (`:49-51`), and is registered
  before it is made the global default (`:80-88`).

## 2. Requirements

R1. A TLA+ entry `formal/tla/LogCaptureFloor/` models the locked `tracing-core`'s callsite
registration, its cached interest, the full rebuild and the level it sets, the switch between one
registered dispatcher and several, and each thread's default, beside the helper's floor install,
its refusal, a capture's dispatcher registration, its default swap and its end, for two threads
(one with no capture, one that captures twice in turn) and one callsite, in the order ADR-492 D1
cites.

R2. Its configuration `MCLogCaptureFloor.cfg` models the code at dev, every switch at dev's value,
and checks `TypeOK` and the three properties of R3 to R5, with no narrowing keyword.

R3. `NoCachedNeverWhileACaptureIsLive` holds: while a capture's dispatcher is registered and live,
the callsite's cached interest is not "never", whichever thread first registers the callsite and
however that registration interleaves with a capture's registration.

R4. `ALineUnderACaptureReachesIt` holds: no line emitted on a thread that holds a capture is
dropped at the level gate or by a cached "never".

R5. `NoInterestIsAskedBeforeTheFloor` holds: no first registration asks a thread's default while
the floor is not the global default.

R6. Each property has one witness configuration with one switch off, which the formal checker
reports as a violation of that property: the floor removed (R3), a floor that hints a level (R4),
and a rebuild or a direct registration outside the helper (R5).

R7. The module covers six items of `tools/log-capture/capture.rs` (`register_floor`,
`with_capture`, `hold_capture`, `refuse_nested_capture`, `register_callsite`, `max_level_hint`),
each digest stamped after the last edit, so a change to any of them stales the entry.

R8. No Rust file, Cargo manifest, lockfile, `config/formal.json`, `scripts/tests` module or
mutation rows file changes, and the census's routed count and every other pin keep their figures.

R9. The entry is checked within the repository's default entry budget.

## 3. Acceptance criteria of SPEC-475

This delivery adds no test, so its acceptance fence names no command. Its three results, A1 to
A3 in section 3a, are each decided by the formal checker over the entry `tla/LogCaptureFloor`,
one property at a time, and the build reports each reading in its hand-back.

```acceptance
```

## 3a. Results the formal checker decides

| id | result | decided by |
|---|---|---|
| A1 | the checker reports `witness/no-cached-never-while-a-capture-is-live.cfg` as a violation of `NoCachedNeverWhileACaptureIsLive`, and reads the property clean in `MCLogCaptureFloor.cfg` (R1 to R3, R6) | the formal checker, entry `tla/LogCaptureFloor`, property `NoCachedNeverWhileACaptureIsLive` |
| A2 | the checker reports `witness/a-line-under-a-capture-reaches-it.cfg` as a violation of `ALineUnderACaptureReachesIt`, and reads the property clean in `MCLogCaptureFloor.cfg` (R4, R6) | the formal checker, entry `tla/LogCaptureFloor`, property `ALineUnderACaptureReachesIt` |
| A3 | the checker reports `witness/no-interest-is-asked-before-the-floor.cfg` as a violation of `NoInterestIsAskedBeforeTheFloor`, and reads the property clean in `MCLogCaptureFloor.cfg` (R5, R6) | the formal checker, entry `tla/LogCaptureFloor`, property `NoInterestIsAskedBeforeTheFloor` |

Each result is one property read alone: the checker reports the property's witness as a
violation and reads the property clean. The red-first record `docs/red-first/SPEC-475.md`
carries one `not red` line per result, naming the witness that is its red, with the witness's
violation line read before the property's clean line. R7 to R9 are read in the build's
hand-back: the covers stamped with no `STALE` left, the diff's paths against section 4, and the
checker's run inside the default budget.

## 4. File manifest

| path | part | change |
|---|---|---|
| `formal/tla/LogCaptureFloor/LogCaptureFloor.tla` | formal | added: the module, its six covers, its three properties and their witnesses |
| `formal/tla/LogCaptureFloor/MCLogCaptureFloor.cfg` | formal | added: the configuration that models the code at dev |
| `formal/tla/LogCaptureFloor/witness/no-cached-never-while-a-capture-is-live.cfg` | formal | added: `Floor = FALSE` |
| `formal/tla/LogCaptureFloor/witness/a-line-under-a-capture-reaches-it.cfg` | formal | added: `FloorHintsOff = FALSE` |
| `formal/tla/LogCaptureFloor/witness/no-interest-is-asked-before-the-floor.cfg` | formal | added: `CensusHolds = FALSE` |
| `docs/specs/SPEC-475-a-model-of-the-log-capture-floor-and-the-callsite-interest-cache.md` | docs | added: this SPEC |
| `docs/decisions/ADR-492-the-log-capture-floor-is-held-by-a-model-of-the-callsite-interest-cache.md` | docs | added: its decision |
| `docs/schematics/log-capture-floor-and-the-callsite-interest-cache.md` | docs | added: the two threads, the cache and the model's actions |
| `docs/red-first/SPEC-475.md` | docs | added: the red-first record |
| `changelog.d/log-capture-floor-model-475.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The `tracing` crates' behaviour beyond the cached interest, the level filter and the default the model reads: span callbacks, the per-call `enabled` answer, event dispatch and a subscriber's `on_register_dispatch`, which the floor leaves as the trait's default (#499 asks for the interest cache).
- The capture's line format and the census's other classes: its token lists, its walk of the sources and the production global default it counts (#461).
- A line emitted inside a dispatcher's own call, which `tracing` drops by design whatever the helper does (#461).
- Logging outside tests, the production global default included (#461).
- More than two threads, more than one callsite, and a nested capture, which the helper refuses (#499 asks for two threads and one callsite).
- Anything only a later run can measure, such as the streak that may later flip a property to blocking (#499).

## 6. Risks

- **The macro's order is read from the tree, not from the `tracing` crate's source.** ADR-492 D1
  takes the level gate before the callsite read from the tree's own emulation of a macro. The
  build re-reads the crate's macro at its cut and stops on another order.
- **The model is an interleaving of atomic steps.** The relaxed loads of the level filter and the
  cached interest are outside it, so a reordering the hardware allows and the model does not is
  not checked. Nothing in this delivery would detect one.
- **A clean reading over unreached actions.** The lock-free path's actions are reached only in
  the witness configurations; a trap in a scratch copy records that the main configuration does
  not reach them, so a clean reading there is the finding, and each witness's violation shows the
  properties can fail.
- **A covered item edited later.** Any edit to the six covered items stales every property of the
  entry until the model is re-read; that is the intended guard.
