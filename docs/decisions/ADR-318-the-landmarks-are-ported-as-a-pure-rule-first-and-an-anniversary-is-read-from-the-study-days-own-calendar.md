---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The landmarks are ported as a pure rule first, and an anniversary is read from the study days' own calendar

## Context and Problem Statement

SPEC-102 ports the predecessor's landmarks (`landmarks.py:compute_landmarks`, `due_today`,
`render_landmark`), one of three issues in one SPEC (#127, #128, #121). #127 reaches three crates:
ingest reads the study days, notifications holds the rule and the texts, and coordination runs the
step and the first run. Four questions needed an answer before any code:

- how #127 is cut into pull requests, given that its second part needs a formal model;
- what the port's rule reads, since notifications reaches neither ingest nor coordination;
- how an anniversary is computed, since the kernel's `StudyDay` has no civil-date arithmetic;
- how the study days are read from the collection.

## Decision Drivers

- The boundary is the compiler: no new `Cargo.toml` edge, `[dev-dependencies]` included.
- Every number is proved against a golden of the predecessor's own function (ADR-012).
- A part that adds no caller, timer or write needs no interleaving model.
- ADR-095 reads what was ever studied from the whole scoped log.

## Considered Options (the alternatives it was chosen against)

- Cut #127 in two: the pure rule, the texts and the study-days read first, the step, the mark and the first run second: chosen, because the first part adds two total functions and a read-only read, has no actor and no write, and ships under a documented FORMAL line of not applicable, while the second part owns the mark, the owed cursor and the router between the fold's writes, whose surface needs a TLA+ model (#127).
- Deliver #127 whole in one pull request: rejected, because it would carry the pure port and the concurrent step together, so the formal obligation of the step would sit on a diff of which most lines are pure (#127).
- Feed the port the study days and the day evaluated as plain values, the epoch days the predecessor's own `analytics.study_day` gives the reviews it keeps: chosen, because notifications then needs neither ingest's study-event predicate nor a second copy of the rule, and the golden records exactly those inputs (#127).
- Re-derive the study days inside notifications from reviews: rejected, because it needs ingest's `is_study_event`, an edge notifications does not have, or a second copy of the rule that can drift (#127).
- Write the first study day with `StudyDay`'s `Display`, add the years to its year and parse the result with `StudyDay::from_str`, parsing day 28 of that month where the parse refuses a Feb 29 of a common year: chosen, because the kernel's own calendar is the one calendar and it equals `landmarks.py:_anniversary` on the golden (#127).
- Add a public civil-date API to the kernel's `StudyDay`: rejected, because it is a shared-kernel change for one caller (#127).
- Do the arithmetic on epoch days with a hand-written leap rule in notifications: rejected, because it is a second calendar beside the kernel's (#127).
- Read the study days by one keyset-paged query over the scoped log, mapping each review id through `StudyDayRule::study_day` into a set kept per day: chosen, because memory grows with the days studied and not with the reviews, and the scope and the study-event predicate are the ones the reader's other reads already use (#127).
- Read the predecessor's whole revlog without the scope: rejected, because it counts a review of a deck out of scope or of a deleted card, which ADR-095 and SPEC-023 R2 exclude from every read of the log (#127).
- Load every review into memory and group afterwards: rejected, because memory then grows with the reviews of a whole history (#127).

## Decision Outcome

Chosen: the two-part cut (this part is the first), the plain-value input, the kernel calendar for the
anniversary, and the keyset-paged scoped read.

- `crates/ingest/src/study_days.rs` is an `impl CollectionReader` block with
  `study_days(&self, rule)`, run through `with_copy`, answering the distinct study days, oldest first.
  Nothing calls it in production in this part; the wiring is #127's second part.
- `crates/notifications/src/landmarks.rs` holds the landmarks, the due rule, the texts and the
  constants, equal to the goldens `landmarks`, `landmark_text` and `landmarks.constants`. The sort is
  by day and then by the key string, so an anniversary key sorts before a study-day key on one day.
- The high-water mark's key is in the golden and is declared and asserted by the second part.

### Consequences

- Good, because the pure port is closed by independent oracles alone, over a hostile population:
  a Feb 29 origin, an anniversary on the day and the day after, the 24th, 25th and 50th study days,
  and the ordinals 11 to 13, 111 and 121.
- Good, because no crate gains an edge.
- Bad, because a study day the predecessor counted from a deleted card or an out-of-scope deck is not
  counted here, so a first study day and the study-day ordinals can differ on one collection; the
  router's once-ever key still raises each key once (SPEC-102 section 10.2).
- Bad, because the port is unused in production until the second part, so a defect in its
  inputs shows only then.
- Vocabulary: `crates/notifications/src/policy.rs` already declares a private `Landmarks` struct, the
  comeback's fresh-start landmarks (`comeback.landmarks` in the policy file). With this part one crate
  names two concepts "landmark". `policy.rs` is not renamed here; the overload is recorded and no
  issue is filed for it.

### Confirmation

- `crates/notifications/tests/landmarks.rs`: A1 to A4 against the goldens.
- `crates/ingest/tests/study_days.rs`: A24 over a synthetic collection.
- Mutation rows S10200 to S10204 on `crates/notifications/src/landmarks.rs`.

## What would make this wrong

- A collection where the scoped log's first study day differs from the one the owner remembers: the
  anniversary would then fall on another day, and the deviation of section 10.2 is what said so.
- A kernel change to `StudyDay`'s `Display` or leap rule: the anniversary reads both, and the golden
  of the Feb 29 origin would go red.

## More Information

Issues #127, #128, #121, #571; SPEC-102 sections 3c and 10; SPEC-023 R2; ADR-012; ADR-095.
