---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The capture helper counts its own captures on each thread, so a missing floor has its own refusal

## Context and Problem Statement

`tools/log-capture/capture.rs` (SPEC-024 section 8) refuses a capture nested inside another on one
thread by reading one fact: whether this thread's default is the floor. A thread that holds no
capture and whose default is not the floor fails the same check, so a missing floor is refused with
the nesting message, which names the wrong cause (#522; #511, note 1).

One such thread is a capture attempted inside a dispatcher's own call. Measured with the pinned
`tracing` and `tracing-core` in a scratch binary: while another thread holds a scoped default, the
inner call reads the default as none; while no thread holds one, `dispatcher::get_default` answers
the global default directly, and the inner call reads the floor. So the helper's floor check is
false inside a dispatcher's own call exactly while some thread holds a capture, with no capture
held on the calling thread. How does the helper tell a nested capture from a missing floor?

## Decision Drivers

- Each refusal names its own cause, so a failing test points at its fix.
- The nesting refusal keeps its message byte for byte, so its test and every reader of it stay.
- The helper is generic over the subscriber it installs, and it is the only file that may name the
  census's install and registration tokens.
- The census's population stays as SPEC-024 A18 states it: 15 routed, none raw.

## Considered Options (the alternatives it was chosen against)

- Chosen: a count of the captures this helper holds on each thread, because only the helper makes
  a capture, so its own count says whether this thread holds one without reading the default.
  `with_capture` raises it for its body and lowers it through a drop guard; `hold_capture` returns
  a `CaptureGuard` that owns the scoped default's guard and lowers the count when it drops.
- (a) One message naming both causes: rejected, because it names neither: a reader of "nested or
  no floor" still has to find out which, and no test can pin each message to its own cause.
- (b) A marker subscriber type tested with `Dispatch::is`: rejected, because the helper is generic
  over the subscriber, so `is` needs a type the helper cannot name for any capture it makes, and
  wrapping every subscriber in a marker changes the type each capture installs.
- (c) Checking the floor's `OnceLock`: rejected, because it says only that the global floor was
  installed once in this binary; a missing floor is this thread's default, which the lock cannot
  see. Inside a dispatcher's own call the lock is set and the default still reads as none.

## Decision Outcome

Chosen option: the count, because it is the one fact that separates the two causes and only the
helper writes it. The refusal reads the count first. Above 0, it refuses with the nesting message,
unchanged. At 0, a default that is not the floor is refused with a message naming the missing
floor, "a capture is refused: the floor is not this thread's default", which does not say
"nested". The count is per thread, so a capture held on another thread is no obstacle, as before.
The one caller that named the returned type, the daemon's `wiring` test module, names
`log_capture::CaptureGuard`.

### Consequences

- Good, because each refusal names its cause, and a test pins each message (SPEC-024 A19).
- Good, because the nesting message, the census's population and every capture call are unchanged.
- Good, because a body that panics inside `with_capture` lowers the count as it unwinds, so the
  thread never counts a capture it no longer holds.
- Bad, because the count is a second record beside the default: a default installed outside the
  helper raises no count, and a later capture would read it as a missing floor. The census refuses
  every such install outside the helper before any test runs it (A18).
- Bad, because `hold_capture` now returns the helper's own type, so a caller that names
  `DefaultGuard` no longer compiles; one did, and it changes with this decision.

### Confirmation

SPEC-024 A19 and A20, in `crates/kernel/tests/log_capture_class.rs`: a capture attempted inside a
dispatcher's own call, while another thread holds a capture, is refused through both entries with
the floor message; a capture made after a held capture dropped is admitted; the nesting test reads
the nesting message unchanged; and the doc of the refusal states the clause "outside a dispatcher's
own call". Three hand rows in the SPEC's band, S02407 to S02409, are each killed by one of those
tests.

## What would make this wrong

- A capture made outside the helper: it leaves the count at 0 with a default that is not the floor,
  and reads as a missing floor. The census refuses that install first.
- A `tracing-core` whose `get_default` read the calling thread's default inside a dispatcher's own
  call: A19's route would read the floor, the attempt would be admitted, and the test would fail
  on an empty refusal, naming the route rather than the helper.

## More Information

SPEC-024 sections 8, 10, 11 and 12; issues #522 and #511 (wording 3); ADR-024.
