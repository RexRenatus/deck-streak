---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The API's health lives on its one loopback listener and is closed at the edge, and its concurrency bound sheds instead of queueing

## Context and Problem Statement

SPEC-025 builds the API service the Mini App calls through Caddy's `/api/*` proxy. The predecessor
served its health routes apart from its API (`server.py:create_server`) and noted that a Mini App backend's public
listener should keep them apart. The rust-service pack requires a concurrency bound and accepts
either a queue or a shed. And the axum testing idiom needs a crate the workspace has not admitted.
Where do the health routes live, what happens to the request past the bound, and which crates does
that take?

## Decision Drivers

- One listener on loopback, reached only through Caddy (ADR-007); every new port is configuration
  and a firewall question.
- Health answers must not leak state to the internet.
- A single owner's traffic is small; a burst past the bound is a fault, and should be visible.
- The memory budget: a queued request holds a connection and its buffers while it waits.
- No crate without an ADR.

## Considered Options (the alternatives it was chosen against)

- Health under `/api/livez` and `/api/readyz` on the API's one loopback listener, with the Caddy block answering 404 for those two paths from outside — chosen: one listener and one port to configure, the probe and the SLO read the same service, and the internet sees nothing.
- A second loopback-only listener for health — rejected because it doubles the listeners, ports and configuration for two routes the edge can close with one matcher.
- Health reachable through Caddy — rejected because readiness describes the service's internals (whether its database is open), which the public need not learn.
- Shed the request past 64 in flight with 503 (tower's `ConcurrencyLimitLayer` inside `LoadShedLayer`) — chosen: the answer is immediate, the connection is released, and each shed is a 503 response event the availability SLO counts.
- Queue the request past the bound until a slot frees (`GlobalConcurrencyLimitLayer` alone) — rejected because waiting requests hold connections and buffers until the 10-second timeout, which spends memory exactly when the service is overloaded, and hides the overload as latency.
- A hand-written semaphore middleware — rejected because tower's layers are the golden path's, tested and understood by the rust-service probe, and the hand-written one would need its own tests for the same behaviour.

## Decision Outcome

Chosen options as above. `tower` is admitted to `[workspace.dependencies]` with the `limit`,
`load-shed` and `util` features: the first two for the bound, `util` for `ServiceExt::oneshot` in
tests. ADR-003 names the concurrency limit among the golden path's layers; that layer is `tower`'s,
which this ADR makes explicit. SPEC-032's Caddy block carries the two 404 matchers.

### Decided at delivery (SPEC-025 §7)

The delivery measured three things this record had assumed, and one it had not named, and
decided each against its alternatives:

- **The bound is `GlobalConcurrencyLimitLayer` inside `LoadShedLayer`, behind axum's
  `HandleErrorLayer` answering 503.** Chosen against `ConcurrencyLimitLayer`, which the options
  above named: the stack is applied with `Router::layer`, which axum 0.8.9 applies to every route
  and every method separately (`PathRouter::layer`, `MethodRouter::layer`), and
  `ConcurrencyLimitLayer::layer` makes a new semaphore each time it is applied, so each route would
  get its own 64 and the service 64 times its routes. Also against wrapping the whole router in
  `ConcurrencyLimitLayer` once: the trace must sit inside the router to read `MatchedPath`, and the
  shed must sit inside the trace to be a logged 503, so the bound cannot sit outside the router.
  The choice still sheds and never queues; A5 sheds a second route while the first holds every slot.
- **Every role opens the database through one opener that holds an exclusive `flock` on
  `deck_streak.db-open.lock` while the kernel's `Db::open` migrates.** sqlx's SQLite migrator
  takes no lock, and two tasks opening one fresh file at once failed at the red-first commit with
  SQLITE_BUSY in 29 of 30 runs of 24 rounds and, once, with `table settings_generation already
  exists`. Chosen against:
  - a `deckstreakd migrate` step ordered before the other units, or migrations applied by one role
    only: the kernel's `Db::open` always migrates, so every other role would need a second,
    non-migrating opener in the kernel, and every unit would need the ordering; a deploy that
    restarted the units together without it would still race;
  - a bounded retry with a reason code: it makes the race rarer and visible, never impossible;
  - a mutex in the process: the roles are separate processes;
  - a lock on the database file itself: closing any descriptor of it drops every POSIX lock the
    process holds on it, SQLite's own included.
  The lock is released explicitly, not only by closing, so a descriptor a child process inherited
  cannot keep it held; the wait for it runs on the kernel's offload, off the async runtime.
- **The request id is copied to the response directly inside the trace**, outside the panic
  catcher, the timeout and the shed, chosen against the innermost place SPEC-025's R4 listed, where
  a 500, a 408 or a 503 made by an outer layer would never carry it (A8's "every response").
- **tower-http is 0.7.1**, the current release, whose layers here keep the API R4 names
  (`TimeoutLayer::with_status_code`); chosen against pinning 0.6 for no measured reason.

### Consequences

- Good, because overload is a counted, immediate 503, never a silent queue.
- Good, because the health routes cost no second listener.
- Bad, because the owner sees a 503 in a burst past 64 concurrent requests; one owner's Mini App
  does not produce such a burst, so a 503 there is a fault worth seeing.

### Confirmation

SPEC-025's A2, A5, A13 and A15; rust-service `rs.concurrency-bound` in the gate; SPEC-032's Caddy
test for the health matchers.

## What would make this wrong

- A caller outside the host needs the health routes (a hosted uptime probe), which would move them
  behind an authenticated route instead.
- Shed requests appear in normal use (the availability SLO's burn shows it), which would mean the
  bound is too low for the real traffic.
- A deploy moves the state directory to a file system where `flock` does not exclude another
  process (a network file system), which would reopen the start-up race the open lock closes.

## More Information

ADR-003; ADR-007; ADR-010; SPEC-025; SPEC-032; the rust-service pack's `service-main.rs.template`;
the predecessor's `server.py:create_server` port note.

Amendment (2026-09-28): passages describing another service's operations were replaced with a
citation of its code, or removed, under the public-text rule (ADR-059).
