# ADR-375: the web client's sync runs the engine's own transport synchronously in its Worker, and the full-sync write's steps live once in the core

- **Status:** proposed
- **Decides for:** `SPEC-364` (#631 part b), the parts ADR-368 left open to part b ("D5, D7 and D10
  are not decided here: parts b and c decide them when they are built"): the transport is decided
  here; the snapshot answer and the page stay part c's. It amends ADR-356 D6, which kept (1,6)
  refused until the full-sync path was measured and the sync's guards were built (D5).
- **Under:** ADR-368 (the choice is one rule in the core), ADR-374 (the sync key sealed, opened only
  in the Worker, sent only by the Worker's credential module to its own origin's sync route),
  ADR-358 D4 (the endpoint guard), ADR-337 (a one-way sync is the owner's tap), ADR-058 and ADR-348
  (the fork, one commit per patch, a tag per pin).

## Context and Problem Statement

The engine refuses its sync transport on `wasm32` (`SPEC-364` M1), runs every sync call inside a
blocking call on its own runtime (M3), reads tokio's clock on the way (M5), starts threads for an
abort and for media (M7), and reads and renames files for a full sync (M9). The core holds the
full-sync choice as types (M15), but every id set those types check is handed in by the adapter,
so the steps whose order is the rule (fetch the server copy, make the backup, re-read the device)
would be written once per client. Each decision below is one ADR-368, SPEC-357, SPEC-363 and
ADR-374 do not settle.

## Decision Drivers

- The engine's protocol stays the engine's: no second copy of its request framing, its status
  handling or its file moves.
- The sync key has one holder in the browser, the Worker (ADR-374 D9).
- Every rule both clients need lives once, in the core (ADR-368 D1).
- Each fork patch is a `cfg` on the target, removable on its own condition, and proven by the
  delivery that adds it.
- The model's abstractions hold: a write is one step, and a backup and the server copy are places a
  review survives in.

## Decisions, and the alternatives each was chosen against

### D1. The request is a synchronous `XMLHttpRequest`, made by the engine's own `wasm32` twin in the Worker

The fork's `wasm32` twin of `zstd_request_with_timeout` sends the request the engine built with a
synchronous `XMLHttpRequest`: the body zstd-encoded in memory, the engine's headers set on it, the
response read as an array buffer with the engine's timeout, decoded by the size header the native
twin reads, and a redirected response refused as the native twin refuses a 308. A synchronous
request is allowed in a dedicated Worker, with a response type and a timeout, and it completes
inside the engine's blocking call, so every future on the path is ready when first polled.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The browser's asynchronous `fetch` under the engine's blocking call | the Worker's event loop does not turn while the engine blocks, so the response never arrives; on a platform with no timers an idle runtime panics (M6) |
| A transport hook the web engine registers with the fork | a public interface upstream never has, and the request's framing, status mapping and size check written a second time beside the native twin |
| A JavaScript import into the Worker's TypeScript that makes the request | the `anki-sync` header, which carries the key, would be built in a second Worker module beside the credential module, and the framing written again in TypeScript |
| Rewriting the engine's sync as sans-IO steps the Worker drives | a rewrite of upstream's protocol inside the fork, with no removal condition |
| Relaying the sync through the service | ADR-374 D1 refused a relay of the key |

### D2. A full sync's files move through SQLite itself on `wasm32`

On `wasm32` an upload reads the closed collection with SQLite's serialize, and a download
deserializes the received bytes into memory, checks their integrity, sets `ls` to `mod`, and
replaces the collection with SQLite's backup interface, in one transaction of the collection's own
journal. The storage pool is the default file system there, so the engine names only paths. The
binding's `serialize` and `backup` features are turned on for `wasm32` alone.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The standard library's file calls | `wasm32-unknown-unknown` has no file system; the calls fail |
| A file hook the web engine registers, backed by the pool's import and export | the pool's import is not atomic, so a Worker that ends mid-import leaves a half collection; and a second public interface in the fork |
| The fork depending on the pool's crate | the engine would carry DeckStreak's storage choice, and its removal condition would be DeckStreak's |

### D3. The write's steps live once in the core, which reads every id set itself

`crates/engine-core/src/one_way.rs` runs each step the model orders and reads each side from its
file: `count` fetches the server copy and reads both sides, `back_up` makes and reads back the
backup, `recheck` fetches a fresh copy, and `write` re-reads the device before `at_write` and the
write. An adapter names paths and holds the stage between the owner's taps; it never passes ids.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Each adapter fetches, backs up and re-reads, and passes the id sets to part a's types | the order of those steps is the rule the model checks, written once per client; and an adapter could pass the device's ids as the backup's |
| A session object in the core that also holds the stage | UniFFI holds an object behind a shared lock and the web engine holds one in a thread-local; the stage holds no rule, so the core gains a second holding idiom per client for nothing |

### D4. The device's backup is SQLite's `VACUUM INTO`, run by the core through the engine's database door

For a download, the core writes the open collection into a new file with `VACUUM INTO ?`, the path
bound, then opens that file on a private engine and reads its ids. SQLite refuses a target that
already holds data, and the statement reads a consistent snapshot of the collection, its log
included, with no close.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The adapter copies the collection's file after a close | a step each client writes, with a close and a reopen around every backup; a copy of a file the engine holds open in exclusive mode with a log is torn unless closed |
| The engine's own packaged backup | it starts a thread and writes a zip through the file system, two more `wasm32` patches |
| SQLite's backup interface from a second connection | the engine's exclusive lock refuses a second connection |
| A fork patch giving the backend a backup-to-path call on its own connection | kept as the fallback if the door refuses `VACUUM INTO` between operations (`SPEC-364` section 6, its first risk): it costs a patch on both targets, where the chosen form costs none; the builder measures the door first and stops if it refuses, and the seat chooses |

### D5. The one-way sync needs both the owner's gesture and the choice's `Write`, and the core builds its request

(1,6) joins the exempt table as `OneWaySync`, its target the collection. `run_exempt` refuses it
with `NeedsTheChoice` before it decodes anything. `run_one_way(gesture, write, auth)` is
`pub(crate)`, reached only from `one_way::write` after that function re-reads the device; it
consumes both tokens and builds the engine's request from the `Write`'s direction, with no media.

This amends ADR-356 D6, which kept (1,6) refused "because the full-sync path is still to be
measured (#620) and SYNC-01's guards are not built (#631, #633)": SPEC-357 measured the path, and
this part builds the guards for the web client. The gesture still names one call, (1,6) on the open
collection, so ADR-356's one-call shape stands: the server copies are fetched on private engines
over empty files, which replace nothing the device holds.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The gesture alone, through `run_exempt` | any caller holding a tap could run a one-way sync with no counts, no backup and no snapshot check |
| The `Write` alone | the containment census holds who may run an exempt write by the gesture's names; a write with no gesture would be the one exempt write the census cannot name |
| Decoding the caller's request and comparing its direction | a field the comparison does not read (the media sequence number) would reach the engine as the caller wrote it |
| `run_one_way` public, so an adapter calls it with the `Write` it holds | part a's types are public, so an adapter could reach a `Write` through `at_write` with ids it chose; `pub(crate)` leaves `one_way::write`, which reads the device itself, the one path |
| ADR-356 D6 kept, the one-way sync left unlisted | the restore after eviction and every full-sync conflict would have no write at all on either client |

### D6. Each delivery pins the fork patches it proves

Part b2's new tag carries `browser-xhr` and `wasm-clock-threads`, which its browser tests prove;
part b3's new tag adds `browser-full-sync-files`, which its tests prove. Each is a fast-forward of
the pin branch with a new tag, and every earlier tag stays (ADR-348).

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| One new tag carrying all three patches in part b2 | the file patch would ship pinned for a whole delivery with no test that reaches it |
| One new tag in part b1 | b1 is native only, so none of the three patches would be proven by the delivery that pins it |

### D7. On `wasm32` the runtime has no time driver, and the sync paths read no tokio clock and start no thread

The fork's runtime builder enables no time driver on `wasm32`; `IoMonitor` keeps no clock there,
the request's own timeout standing for its stall watch; the full-sync progress monitor never
ticks; a sync's abort is sent inline; a media sync in the background refuses. The web transport
forces no media until part d, so the refusal is never reached in use.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A time driver backed by the browser's timers | tokio has none for this target without unstable features; and a timer cannot fire while the engine blocks |
| Leaving the clock and catching its panic | a panic in the web engine aborts the module, and the Worker loses its engine |
| Replacing tokio's clock with a browser clock | tokio's clock is not replaceable, and its timers would still need a driver |

### D8. A server copy or backup is a new file, and nothing the choice makes is deleted

Every copy and backup is fetched or written into a file that holds no row; the core refuses
otherwise. No step of part b removes one; part c decides how many are kept and where.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Fixed slots overwritten by each choice | the next count would overwrite the last upload's backup, a place the model's `NoReviewLost` counts a review in |
| Removing the copies once the write lands | the same: the counted copy is an upload's backup |

### D9. The normal sync is admitted on the web alone, with no media

(1,5) joins the ordinary table `web: true`, `native: false`, and the core refuses a request that
asks for media.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| `native: true` now | it admits a call for the iOS client before its sync is designed (#633) |
| Media allowed | a media sync starts a thread on `wasm32` and fills a directory nothing reads until part d |

### D10. The page asks for persistent storage at every session start

The study page calls `requestPersistence` each time a session starts. The storage interface's
`persist` is exposed to a window, not to a Worker, so the page asks.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The Worker asks | `persist` is not exposed in a Worker |
| Asking once and remembering | a browser may grant later after engagement; asking each session costs nothing when granted |

### D11. The web side names its two sync pairs beside its study calls, and `run_method` never admits them

The web engine's `study.rs` gains `SYNC_CALLS`, the login (1,3) and the normal sync (1,5), and the
parity guard compares the core's web column with `STUDY_CALLS` and `SYNC_CALLS` together.
`run_method` keeps admitting `STUDY_CALLS` alone; part b2's sync exports are the only web path to
the two sync pairs.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The two pairs added to `STUDY_CALLS` | `run_method` would then run a sync with the caller's bytes, a second path beside part b2's sync exports, and two web-engine tests that pin `STUDY_CALLS` as the review's pairs would be rewritten |
| (1,3) and (1,5) left `web: false` until part b2 | (1,5) is `native: false`, so b1 could prove its endpoint guard and its media refusal on no transport at all |
| The sync service exempted from the parity guard | an equality guard with an exemption is a weaker guard, and the exemption would outlive the part that needed it |

## Consequences

- Good, because the engine's protocol, its status handling and its file moves stay the engine's,
  each change a `cfg` on `wasm32` with its own removal condition.
- Good, because the key reaches the network only through the engine's header, built inside the
  Worker's module, after the credential module's `forSend`.
- Good, because the iOS client calls the same driver with its own paths, and writes no step of
  the choice.
- Bad, because a synchronous request blocks the Worker for the sync's length: study waits while a
  sync runs. The page starts a sync at a session's start and end, not during review.
- Bad, because each choice leaves files in the pool until part c's retention removes them, and the
  pool must be reserved before each.
- Bad, because the fork grows by three patches, each needing an upstream answer to retire.
- Neutral: a stall now ends at the request's timeout, not at the engine's inactivity watch.

## What would make this wrong

- A browser that drops synchronous requests from dedicated Workers: D1 would need the sans-IO
  rewrite it rejected.
- The engine's database door refusing `VACUUM INTO` between operations: D4 falls to its fallback.
- An upstream release that builds its sync for the browser: D1, D2 and D7's patches retire.
- A retention rule in part c that removes a backup with no reason the model accepts: D8's place
  would be lost, and `NoReviewLost` would need a bound on how long a backup is a place.
- A part b2 design that routes the sync through `run_method`: D11's `SYNC_CALLS` would fold into
  `STUDY_CALLS`, with the web-engine tests that pin it.
