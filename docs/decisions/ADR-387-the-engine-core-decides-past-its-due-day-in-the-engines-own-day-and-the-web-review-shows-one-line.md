---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-387: the engine core decides "past its due day" in the engine's own day, and the web review shows one line

Decides SPEC-376 (issue `#713`). The grading decision's late-review rule says a late review is
told plainly that it does not count toward the streak. The streak counts any study review on the
study day it was made (SPEC-076 R2), so this record decides what the review can say truthfully,
which clock decides "past its due day", where that is computed, and how the web review carries it.

## Context and Problem Statement

A card's due day counts in the engine's day: whole days since the collection's creation, turning
at the collection's rollover in the device's zone. The streak counts in the kernel's study day: a
fixed offset and a rollover hour, with no zone database (`crates/kernel/src/study_day.rs:103-113`).
The two agree exactly when both days end at the same instant
(`crates/ingest/src/skip_write.rs:929-939`). The web review's card view carries no due day
(`web/app/src/lib/engine/protocol.ts:101-114`), the engine core is the one client-side holder of
the engine and depends on no crate of this workspace (`crates/engine-core/src/lib.rs:1-34`), and
the native app has a review screen and no localized strings.

## Decision Drivers

- The line must be true whenever it is shown (the late-review rule says "plainly").
- Every client reads one answer, and the engine core may gain no workspace dependency (ADR-356 D4).
- The review works offline: no answer may wait on the server.
- No streak rule, write, table pair or store changes.

## Considered Options (the alternatives each was chosen against)

### D1. The due-day boundary

- Chosen: the engine's own day, because a card's due counts in that frame, the engine computes it on the device with no server and no new dependency, and the line's claim is about the day the card was due, which is the engine's day.
- Chosen against: the kernel's study day, because the core cannot reach the kernel (ADR-356 D4), the device does not hold the server's offset and rollover, and a due day that spans two study days, as it does whenever the two rollovers differ, has no single study day to convert to.

The case where they differ, named: when the two days do not end at the same instant, for the hours between the two rollovers of each day (3 hours a day for a zone 3 hours east of the rule's offset with both rollovers at hour 4), a card due on the engine's previous day reads past its due day while the kernel still counts the study day that overlaps it (SPEC-376 section 1b, risk recorded in section 6).

### D2. Where "past its due day" is computed, and how the review receives it

- Chosen: in the engine core, as one pure rule over the card's queue, due, home deck due and home deck and the engine's day, with the engine's day read by one call the core holds; the web engine puts the answer on the card view as `late`, because both clients reach the engine only through the core, so the native review will read the same answer.
- Chosen against: the server's store, because the review is offline and the server's answer would arrive after the card is shown, or never.
- Chosen against: each client computing it, because two copies of one rule drift, and the page holds neither the card's due nor the engine's day.
- Chosen against: admitting the engine's timing of today as an adapter pair in the core's table and the web engine's study table, because an adapter needs only the answer, and a pair would widen two closed tables for a read the core can hold, as it holds the undo status (`crates/engine-core/src/dispatch.rs:238`).
- Chosen against: reading the answer from the card's next states (elapsed days beyond scheduled days), because that holds only for a review state and reads a figure the engine may derive from the last review rather than from the due day.

### D3. The line

- Chosen: `This card was due on an earlier day, so this review does not count toward the streak for that day.`, one sentence, above the card and below the status line, on both sides of the card, because it is true in every case the line shows: the review is made on a later study day than the card's due day, so it cannot count toward that day.
- Chosen against: `This review does not count toward the streak.`, because it is false whenever the card is reviewed on a study day the fold has not yet settled: SPEC-076 R2 advances the streak for any study review on its own day, an overdue card's included.
- Chosen against: a line naming the due date or the days overdue, because the issue asks for one plain line, and a number reads as blame.
- Chosen against: the status line (`role="status"`), because it announces refusals and would be re-announced for every late card; the line is part of the card's content, read in order with it.

The 7 locales carry it under one key, `study_late_review`, each in the locale's own word for the streak as its tagline spells it; the build writes each translation, and a census holds the key, the word and that no locale but `en` holds the English text.

### D4. The native review

- Chosen: named out of scope, with its own issue, because the native app holds no localized strings at the base, so "in every locale" there needs a string catalog first, and the open native pull request touches the same core and adapter files.
- Chosen against: adding the line to the native review now in English only, because the issue asks for every locale, and an English-only line would ship a known gap.
- Chosen against: building the native string catalog in this delivery, because it is a new surface for every native string, larger than this issue.

### D5. The tests and the model

- Chosen: red-first tests per criterion, the rule's boundaries on both sides of the due day in the core's own crate, the engine day against the engine's own timing as an oracle, a source census for the `wasm32`-only view, and a locale census with planted refusals; no model, because no actor, store or write path is added and the clock only advances, so a line true when shown stays true.
- Chosen against: a TLA+ model of the card call and the rollover, because the one interleaving, the day turning while a card is shown, can only leave a true line or no line, never a false one.
- Chosen against: a Lean proof of the rule, because no recorded failure motivates it and its arms are few enough for the tests to cover every boundary.
- Chosen against: rows whose killers read the web sources' text, because StrykerJS mutates each changed production file under `web/app` on the pull request.

### D6. The shape

- Chosen: one delivery under SPEC-376 and ADR-387, because the work is a client rule and a line, in the engine core, the web engine and the page, none of which SPEC-076 or ADR-076 governs.
- Chosen against: an insert-only amendment of SPEC-076 and ADR-076, because those decide the server's streak rules in the streaks and coordination contexts, which this delivery does not change, and an amendment would put engine and page files under a server record.

## Decision Outcome

D1 to D6 as chosen above. The engine core answers whether the shown card is past its due day in
the engine's own day; the web engine puts that answer on the card view; the web review shows one
line, true whenever it shows, in every locale; the native review reads the same rule when its own
issue lands.

## Consequences

- Good: the line never says a review does not count when the streak would count it.
- Good: one rule, in the core both clients reach, with no new dependency, table pair or store.
- Bad: the line speaks about the day the card was due, not about today, so a learner who expects
  "this review is wasted" is told something narrower, and true.
- Bad: where the two clocks differ, the line can name a day the kernel still partly counts
  (D1's named case).
- Neutral: the native review shows no line until its own issue.

### Confirmation

SPEC-376's A1 to A11, the censuses' plants, the band's rows by their killers, and the
`mutation-web` verdict on the pull request.

## What would make this wrong

- If the owner rules that a late review is one that syncs after its day has settled, the line
  belongs to the sync's outcome, not to the review screen, and D2 and D3 are decided again.
- If the owner rules that an overdue review must not count toward today's streak, that is a change
  to SPEC-076 R2, decided there first, and only then may the line drop "for that day".
- If the kernel's rule and the engine's day are made to differ by design, D1 is decided again,
  with the server's offset and rollover served to the client.
