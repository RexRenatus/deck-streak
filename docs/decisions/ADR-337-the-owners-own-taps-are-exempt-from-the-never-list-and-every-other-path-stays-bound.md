---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The owner's own taps in the study client are exempt from the never-list, and every other path stays bound

## Context and Problem Statement

ADR-301 (a) lists eight writes that no write class makes, at any rung, and that no owner approval
admits. The study client is a real Anki client and the owner's primary one (SPEC-334 R2), and the
owner chose AnkiMobile parity for their own taps (the owner's answer ANK-03). Each of these taps
meets an entry: undoing an answer after it has synced (1), Forget (2), deleting a preset (3), the
one-way sync choice (4), switching a preset's scheduler (5), set due date (6), changing a note's
type (7) and deleting a reviewed card or note (8). Under ADR-301 as written, none of them can be
built. The owner's signed ruling,
`docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`, admits them on conditions. How does the
build carry that ruling so that the exemption reaches the owner's own hand and nothing else?

## Decision Drivers

- The exemption covers a write the owner makes by an explicit gesture in the study client, on
  what that gesture names, shown before it is made and confirmed where Anki asks.
- Every other path stays under entries 1 to 8 verbatim: every write class, agent, batch,
  background or scheduled job, and every repair or reconciliation of sync.
- The ruling demands proof that no non-UI caller reaches an exempt function, not a convention.
- Nothing calls an exempt function before the ruling is on `dev`.

## Considered Options (the alternatives it was chosen against)

- Exempt the owner's UI gestures, carried by a token only the UI layer can construct and proved by a containment test — chosen because it is the ruling's exemption, and the compiler plus one test make "only from the UI" a property of the build rather than of review.
- An owner-acts category that keeps the never-list's entries — rejected because it leaves the client short of AnkiMobile parity, which the owner declined at ANK-03.
- Owner approval of a write class's batch as the route to these writes — rejected because ADR-301 (a) refuses it and the ruling keeps that refusal: approval is not a tap.
- The exemption by convention and code review alone — rejected because a review cannot prove that no agent route, server job or sync repair reaches an exempt function, and the ruling requires that proof.

## Decision Outcome

Chosen option: the owner's UI gestures are exempt, through a token only the UI layer can
construct, and a test proves the containment. The owner-taps ruling is on `dev`, so this ADR is `accepted`.

- **The engine core's allow-list.** The engine core reaches the engine's backend only through an
  allow-listed dispatcher. The exempt functions are the backend calls that make writes 1 to 8:
  undo after sync, Forget, deleting a preset, a one-way upload or download, switching a preset's
  scheduler, set due date, changing a note's type, and deleting a reviewed card or note.
- **`OwnerGesture`.** Each exempt function takes an `OwnerGesture` value. Only the study client's
  UI adapters, the SwiftUI screens through FFI (ADR-335) and the web client's screens through the
  Worker (ADR-336), can construct one, and each construction names the one card, note, preset or
  collection the gesture acts on. No write class, agent route, server job or sync repair has a
  path that constructs one.
- **The containment test.** A test in the engine core proves that no non-UI caller reaches any
  exempt function: it enumerates every caller of each exempt function in the workspace, reports
  the count examined, and refuses any caller outside the UI adapters; a planted non-UI caller is
  refused by name.
- **Shown, then confirmed.** Before an exempt write, the client shows what it changes, and it asks
  for confirmation wherever Anki asks for one.
- **Entry 4, the one-way sync,** keeps SPEC-334 R8's guards: what each side loses, an on-device
  backup first, and for an upload a check that the server's offsite snapshot exists.
- **Entry 5, the scheduler switch,** is the owner's tap on one preset (ADR-338). Every other
  experimental model only reorders cards already due and stays bound by entries 5 and 6
  (ADR-339).
- **Order.** Nothing calls an exempt function before the ruling is on `dev`. ADR-301 keeps its
  text; the delivery that builds the first exempt tap appends a note at ADR-301's never-list
  naming the ruling.
- **CHARTER 4.** This ADR carries the app-surfaces ruling's amendment of CHARTER 4 for the owner's
  own answers and gestures (`docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`), with ADR-338
  and ADR-339 for the scheduler it names.

### Consequences

- Good, because the study client reaches AnkiMobile parity for the owner's own hand, which the
  owner chose.
- Good, because the exemption's reach is a type the UI layer alone constructs, checked by the
  compiler and by one test on every change.
- Bad, because eight kinds of write that ADR-301 refused outright now exist in the codebase, and
  a mistake in the UI layer itself is not caught by the containment test.
- Bad, because every new exempt function must join the allow-list and the test's population in
  the same change.

### Confirmation

- The containment test's examined count and its planted non-UI caller, under the gate.
- Each exempt tap's own tests: the preview shown, the confirmation asked, the one target written.

## What would make this wrong

- A legitimate need for an automated path to one of the eight writes, which would need a new
  ruling and a declared write class, never this exemption.
- A UI adapter that constructs an `OwnerGesture` without an owner's gesture, for example on a
  timer or at launch; each adapter's tests assert that the token comes from a gesture handler.

## More Information

- `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`; ADR-301 (a) (the never-list).
- SPEC-334 (R7, R8); ADR-335, ADR-336, ADR-338, ADR-339.

## Amendment: the undo of an unsynced answer (SPEC-371)

ADR-382 amends the Decision Outcome (`:41-44`), which names the exempt functions as the backend
calls that make writes 1 to 8. The exempt table also holds Undo, for the review's own last answer
while it has not synced. Whether that undo is the never-list's entry 1 is read two ways: the
owner-taps ruling's table maps entry 1 to undoing an answer after it has synced, and ADR-361 D1
says the never-list does not name undo. ADR-382 builds to the stricter reading: the undo meets all
three of the ruling's conditions whichever reading holds.

## Amendment: the undo of an unsynced bury or flag (SPEC-383)

ADR-397 amends the Decision Outcome (`:41-44`), as SPEC-371's amendment did. The exempt Undo also
holds the undo of the review's last bury or flag while it has not synced. It meets the owner-taps
ruling's three conditions as the undo of an answer does: it is reachable only from the review, it
writes only on the card the gesture names, and it is shown, with what it changes, before it is
made.
