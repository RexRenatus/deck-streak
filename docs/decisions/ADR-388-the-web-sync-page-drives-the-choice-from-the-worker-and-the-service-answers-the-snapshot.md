# ADR-388: the web sync page drives the full-sync choice from the Worker, the full sync's files move through SQLite in the fork, and the service answers whether a sealed snapshot exists

- **Status:** proposed
- **Decides for:** `SPEC-377` (#631 part c), the parts ADR-368 left to part c (D5: the snapshot
  answer; D10: the page) and the parts ADR-375 left to the delivery that proves them (D2's fork
  patch, under D6: each delivery pins the patches it proves). Part c1 writes D1 to D13; part c2
  builds D1, D2, D3, D14, D15 and D16 and amends this record insert-only.
- **Under:** ADR-368 (the choice is one rule in the core), ADR-375 (the transport and the write's
  steps), ADR-374 (the sync key sealed, opened only in the Worker; D10: the sign-in form posts to
  the Worker once), ADR-337 (the owner's own tap is the exempt path), ADR-058 and ADR-348 (the fork,
  one commit per patch, a tag per pin).

## Context and Problem Statement

A full sync cannot run in the browser at `dev`. The engine's upload reads the collection's file
with the standard library, its download writes a temporary file and renames it over the collection,
and its progress monitor ticks a timer (SPEC-377 M1 to M4); `wasm32-unknown-unknown` has none of
them. Every choice reaches the download before the owner's first tap, because the counted server
copy is itself a download (M5). Beside the fork, the core probes the file system twice on the
choice's paths (M8), and on `wasm32` both probes answer without failing and answer wrongly. The
page has no sync screen, and nothing on it calls the sync (M12 to M17).

## Decision Drivers

- The full-sync choice stays one rule in the core (ADR-368): no second copy of its steps, its
  counts or `before_upload` anywhere else.
- Nothing the choice makes is deleted by the choice (ADR-375 D8), and nothing is written over the
  open collection except the confirmed write.
- The page derives nothing; the Worker is the only holder of the sync key (ADR-374).
- The snapshot answer is checked, never the owner's word, and no credential reaches the page.
- Study is never interrupted or held by a sync.
- Public text names roles, never a host, a credential, a store or a provider.

## Decisions, and the alternatives each was chosen against

### D1. The snapshot answer is a route on the service (outline Dc1, ADR-368 D5)

An owner-session route on the service lists the archive's prefix with a list-only credential,
named only by its role, and answers `{found, age}`; it never reads an object, and it never names one.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A receipt directory shared with the archive job | a second writer to keep in step with the archive, and a receipt can claim an upload that later failed |
| The archive job posting each upload to the service | a new inbound path into the service, for a fact the listing already holds |
| The browser reading the store | a store credential in the page |
| A route on the sync server | a fork patch to the server, for a question about the service's own archive |
| The owner's word | no check at all; the model's `SnapCheck` would be decided by a tap |

### D2. Found means a sealed snapshot exists; its age is shown, and no bound is set (outline Dc2)

`found` is a sealed archive and its sealed manifest at one stamp. The screen shows the newest such
stamp's age before the upload's tap.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A freshness bound | the archive's cadence would decide uploads, and one missed run would block every upload; a bound would enter the model first |
| No age shown | the owner could not judge an old snapshot |
| An archive without its manifest counted as found | the archive cannot be checked whole without its manifest |

### D3. Retention keeps the newest three of each kind and never removes the newest (outline Dc3)

Every backup and counted copy is kept until three newer ones of its kind exist; the newest of each
kind is never removed; each removal is named on the screen; any backup can be exported first.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Keep everything | the browser's quota ends it silently, by eviction of the whole origin |
| Keep one | a second choice before the owner checks the first loses the first's backup |
| Remove after a successful sync | the server then holds the only copy |
| Remove by age | a device idle for a long time would lose every backup at once |

### D4. The sync runs at a session's start and end, never during review (outline Dc4)

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A background sync every few minutes | the synchronous request holds the Worker, so study would stall mid-review |
| A sync after every answer | the same stall, once per card |

### D5. The page derives nothing (outline Dc5)

Counts, the offer, the status, the snapshot's age and the unsynced figure come from the Worker's
replies, and the page shows them.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The page computing a loss, a status or an offer | a second copy of a core rule, free to drift from it |

### D6. The full sync's files are cured by one fork patch, `browser-full-sync-files`, under a grant

ADR-375 D2 decided the cure; this part proves it. One commit on the fork's pin branch: on `wasm32`
the upload serializes the closed collection, the download deserializes the received bytes, checks
their integrity, sets `ls` to `mod` and replaces the collection with SQLite's backup interface in
one transaction through the default file system, and the progress monitor never ticks; the
binding's serialize and backup features are on for `wasm32` alone, and the native engine is
unchanged. A new tag pins it, and no tag moves. A wide-open trial records every unsupported call on
the choice's browser path before the commit, so the patch covers what was measured. The fork write
is made only under main's grant, scoped to these calls.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A path inside this repository over the engine's public transport | measured: the file calls sit inside `pub(super)` engine functions, and `before_upload`, eleven statements an upload runs first, is `pub(crate)`; a bypass would copy an engine rule and skip the core's one-way request |
| A file hook the web engine registers, backed by the pool's import | ADR-375 D2: the import is not atomic, and it is a second public interface in the fork |
| A target with a file system | the web engine's build, its Worker and every earlier `wasm32` patch assume this target; a second target doubles the pin |
| No full sync on the web | an evicted collection could never be restored, and the owner could never choose a direction |

### D7. Where files live is the adapter's: the core asks a `Files` port

The core reads whether a path holds a file and whether two paths name one file through a `Files`
port its dispatcher keeps. The standard library's port is the native default, with today's
behaviour. On `wasm32` the default is a fail-closed port that names every choice path as the open
collection, until an adapter installs its own; the web engine installs the pool's at start.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Leave `exists()` and `canonicalize()` in the core | on `wasm32` they answer "absent" and "no other spelling" without failing, so the rule against writing into a file with rows or the open collection holds natively and silently not in the browser |
| The core naming the pool's crate | the core would carry a browser storage choice, and every client's core would build it |
| The web engine checking each path before it calls the core | the rule would live in two places, and a caller that forgot the check would write |
| Probing a path with a private engine's open | the open creates the file, so the backup's `VACUUM INTO`, which needs an absent or empty file, would then refuse |
| A standard-library default on `wasm32` | the browser would get the silent wrong answer this decision exists to remove |

### D8. Every choice file is a new pool name the web engine mints; the page passes no path

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The page passing a path | the protocol would carry a path, and a page could aim a fetch at the collection |
| Fixed names per kind | a second choice would land on the first's backup, against ADR-375 D8 |
| Names without a reserve | the pool's capacity is fixed at install, so the next file would fail mid-choice |

### D9. The web engine holds a choice's stage between taps; a Worker that ends is the model's `Cancel`

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The page holding the stage | the page would hold engine state and could hand back a forged `Ready` |
| Persisting the stage in the pool | a stage read after a restart could write without a fresh count |

### D10. The Worker reads the snapshot answer from the service's route

Before an upload's confirm, the Worker fetches the route with the owner's session on its own
origin; an absent route, a refusal or a network failure is `unknown`, and `unknown` refuses the
upload as `not found` does.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The page passing `found` | the page could forge it, and the page derives nothing (D5) |
| The web engine fetching it | the engine's transport is the sync's own; a second route there is a second transport |
| `unknown` admitting the upload | the model's `SnapshotBeforeUpload` would no longer hold |

### D11. The sync screen is a route of its own, `/sync`

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A settings screen | none exists at `dev`; a screen of panels for one panel |
| A dialog over the study page | the choice would interrupt review (D13) |

### D12. A session posts one sync after its open and one before its close, and awaits neither

The Worker's one queue orders them with study requests, and the request's total timeout stays the
engine's stall duration (ADR-375 D19).

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Awaiting the sync before study | an offline or slow server would hold study |
| A separate queue for the sync | two writers to one collection |
| A longer timeout for a full sync | the timeout bounds a hung request, and a full sync's transfer stays inside it or fails writing nothing |

### D13. A full sync required is a notice that links to `/sync`, never an interruption

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A dialog at the session's start | it interrupts the session, and a choice made mid-session risks the session's own reviews |
| Saying nothing | the owner would never learn the normal sync stopped |

### D14. An export is the closed backup's collection file, downloaded as a file named by kind and age (part c2)

The bytes reach the page by transfer, and the page names a backup only by an id the Worker listed.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The engine's packaged backup | ADR-375: it starts a thread and writes a zip through the file system |
| Sending the backup to the service | the learner's own data would leave the device without the owner's tap |
| The page naming a pool path | a path in the protocol (D8) |

### D15. Retention is one pure rule in the core, run by the Worker only when no choice is held (part c2)

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Retention in the page | the page derives nothing (D5) |
| Retention in the Worker's TypeScript | a second copy beside the core's rule |
| Retention while a choice is held | it could remove a place the model's `NoReviewLost` counts for the choice in flight |

### D16. The route lists through a port; the composition root wires a configured list command (part c2)

The service's crate holds a lister port and answers `unknown` when none is wired. The daemon wires
a list command from settings, run with arguments and no shell, bounded in time and output, under a
list-only credential named only by its role.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| A store client library in the service | a new dependency and a provider named in the tree, for one list call |
| A command line built as a string | a shell would parse it, so a setting could carry a second command |
| A credential with read access | the answer needs a listing, never an object |

### D17. A backup's age is the web engine's record of when it first listed the file (part c2)

The pool keeps no file times, and part c1 mints the first free number, so a number freed by
retention is minted again for the newest file. The web engine records the time it first lists each
backup or server copy as an empty pool entry beside it, and orders by that record, then the number.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| The minted number as the order | a number freed by retention is minted again, so the newest file can carry the lowest number |
| The pool's file times | the pool keeps none |
| The collection's own modified time inside each file | a read outside the core's fixed set of statements, and it dates the collection, never the copy |
| A time in the minted name | part c1's mint, pinned by its census and its rows, would change |

### D18. Retention runs before each backup list, which the screen reads after every write or cancel (part c2)

The Worker's choice is part c1's and stays as it merged; the sync screen asks for the list at its
start and after every write or cancel, and the Worker runs retention first. The web engine refuses
retention while a stage is held, so it never runs inside a choice.

What it was chosen AGAINST:

| alternative | why it lost |
|---|---|
| Retention inside the choice's confirm and cancel | it rewrites part c1's choice, which this part may not change |
| A timer in the Worker | it could run while the owner reads counts, and only the stage check would hold it back |

## Consequences

- The fork gains one commit and one tag, and the root manifest's `[patch]` moves to it; ADR-058
  gains a row. The native engine's tests read equal before and after.
- The core gains a port with one native and one fail-closed default; every other client installs
  its own when its sync lands.
- The protocol gains four operations, none carrying a path, an id set or a snapshot answer.
- The service gains one owner-session route in part c2, off until a lister is wired.
- `formal/tla/FullSyncChoice` is unchanged: no covered anchor moves.

## What would make this wrong

- The wide-open trial finds an unsupported call the patch cannot cover in the fork: D6 is then
  amended before any fork commit.
- The pool's capacity cannot hold three of each kind beside the collection: D3's three is lowered
  on the measured quota.
- An owner wants uploads refused past an age: D2 then gains a bound, in the model first.
- The service cannot run a command under its sandbox: D16 then needs a different lister, decided
  before part c2 is built.
