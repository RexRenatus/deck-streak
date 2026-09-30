---
status: "accepted"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The law streak is evaluated on every study day, so it decays when law study stops

## Context and Problem Statement

The law track keeps its own streak: consecutive study days with at least one law review, bridged
by declared skip days, with no freezes and no comeback (the predecessor's feature `law-streak`,
#82). The predecessor computes it with `analytics.py:bridged_streak`, which answers correctly for
any day it is asked about, but its caller, `pipeline_layers/digests.py:DigestsLayer._update_law_streak`,
returns early on every day without a law review. So the stored law streak is rewritten only on law
study days: once law study stops, it keeps its last value until the next law study day, and every
surface that reads it (the law block that leads the home screen, `/today`, the digests) shows a
streak that has already ended. The schematic `docs/schematics/streaks-and-governor-state-machine.md`
already records that this defect is fixed, not ported. How does DeckStreak keep the law streak true
without inventing a rule the predecessor never had?

## Decision Drivers

- The eleven anti-goals include no dishonest copy (CHARTER 10): no surface may show a streak that
  has ended.
- The math ports verbatim (CHARTER 8): the streak's value must still be the predecessor's own
  function, proved by its golden.
- The law track has no freezes and no comeback by the predecessor's design: the caller's own
  documentation keeps those mechanics on the language track.
- Law is primary: the law block leads the home screen whenever there is law activity (#134, #69),
  so a stale value would lead with a false number.
- The recompute settles each study day once, in order (ADR-071), so the streak can be evaluated
  for every day, with law study or without.

## Considered Options (the alternatives it was chosen against)

- Evaluate `bridged_streak` on every settled study day and on the current study day, law study or not — chosen: the stored value is always what the predecessor's own function says the law streak is on that day, and only the caller's early return is dropped.
- Copy the predecessor's caller, which writes the law streak only on law study days — rejected because once law study stops the stored value never decays, and the law block would lead the home screen with a streak that has ended (dishonest copy, CHARTER 10).
- Store only the longest law streak and compute the current one on every read — rejected because every reader would need the window's law study days, which only the recompute reads, and the settle's later steps read the current value too.
- Decay the law streak through the language streak's gap classifier and freezes — rejected because the law track has no freezes by the predecessor's design, and borrowing the classifier would invent a freeze rule the owner never had.
- Zero the law streak only when a break marker is written — rejected because it needs a state the predecessor lacks and still shows a stale value between the last law study day and the marker.

## Decision Outcome

Chosen option: "Evaluate `bridged_streak` on every settled study day and on the current study
day", because it keeps the predecessor's function and its golden unchanged and removes only the
early return that made the stored value lie.

- At each recompute, the fold (ADR-071) sets the law row's current value to
  `bridged_streak(law study days, day, declared skip days)` for every study day it settles, in
  order, and then for the current study day. The longest is the larger of itself and each current
  value; the last law study day is the latest day with a law review.
- `bridged_streak` treats the day it is asked about as unfinished: a day with no law review yet
  keeps the run that ended the day before, and the study day after a missed day that was not a
  declared skip reads zero until a law review starts a new run.
- The law row's freezes stay at the start value, and nothing consumes or grants one.

### Consequences

- Good, because the law streak on every surface equals the predecessor's own function for that
  day, proved by `goldens/bridged_streak.json`.
- Good, because no state and no rule is added: only a call site changes.
- Bad, because during side by side the predecessor's bot keeps showing its stale law streak while
  DeckStreak shows a smaller, true one; the side-by-side verification treats this row as a
  recorded divergence (ADR-011, #62).
- Bad, because a law review that reaches the sync server after its day has settled does not rejoin
  the law streak, as for the language streak (SPEC-076, section 6).

### Confirmation

SPEC-076's acceptance tests: the law streak equals the golden of `analytics.py:bridged_streak` on
every settled and current day (A9), it reads zero after a study day with no law review and no
declared skip (A10), and no freeze is spent from or granted to the law row (A11); and the mutation
row that restores the early return (S07607).

## What would make this wrong

- The owner decides the law block should keep showing the last law run while law study pauses:
  then the view adds that value beside the streak, and the rule stays.
- The owner gives the law track freezes: then the law streak moves to the language streak's gap
  classifier with its own freezes, by a new decision.

## More Information

SPEC-076; ADR-071; the predecessor's `analytics.py:bridged_streak` and
`pipeline_layers/digests.py:DigestsLayer._update_law_streak` at `27ee2bc`;
`docs/schematics/streaks-and-governor-state-machine.md` and
`docs/schematics/streaks-governor-and-freezes-in-w3.md`; #82.

## Amendment, 2026-09-29: the relight's grant and celebration meet the fold's one transaction

The recompute's fold holds one write transaction while its steps run (ADR-071). The relight's XP
grant and its celebration cannot each open a writer of their own inside it. The decision, made on
the orchestrator's ruling at the third dispatch (SPEC-076, section 12):

- The relight's XP is written on the fold's connection, through a connection-level `grant_on` that
  carries the grant port's two queries unchanged, in phase 3, so the XP is in the day's base before
  the derived bonuses and the mint read it in the same recompute. Chosen because it keeps the
  grant port's once scope and the same-recompute base.
- The celebration is routed after the fold's commit, under the policy's `celebration` kind with the
  key `relight:<epoch day>`, on every settle that qualifies. Chosen because the router's once-ever
  dedupe gives one send per episode and a crash between the commit and the route is recovered at
  the next recompute.
- Rejected: a grant after the fold has run, because R18's same-recompute base would not hold.
- Rejected: a second write inside the fold, because the one writer would deadlock against the fold's
  held transaction.

## Amendment, 2026-09-30: the counter loops are bounded by the range they read

The silence walk and the replay, fold and bridge loops move a counter that a mutant can stop
moving, so a mutant of the counter never ended and cost a shard its whole budget. Each loop now
runs over a counted range: the walk over `0..=SILENCE_WALK_CAP_DAYS`, the others over the epoch
days they read. The answer is unchanged, proved by a generated population against a test-only copy
of the earlier walk.

- Rejected: leaving the loop and raising the job's timeout, because it is a workflow setting the
  owner alone changes and it hides the spinning mutant instead of removing it.
- Rejected: a per-test timeout, because the verdict would then depend on the machine's speed.
- Rejected: converting `open_lapse`, because it ends by `checked_sub` over a finite window and a
  conversion would change its answer at the smallest epoch day.

## Amendment, 2026-09-30: the open lapse walks a counted range too

The amendment above rejected converting `open_lapse`, because the loop ends by `checked_sub` over a
finite window and a conversion would change its answer at the smallest epoch day. The first reason
holds for the loop and not for its mutants: `while number < window_start` does not end for a today
before the window's first day (today 0 with a window from day 1 gave no answer in 5 seconds), which
is the spinning mutant the amendment exists to remove. The second does not hold: a conversion that
answers nothing at the smallest epoch day, as the `checked_sub` step did, answers what the earlier
loop answered over 1,765,680 generated cases, 209,340 of them at the smallest epoch day (SPEC-076
A41). `open_lapse` now walks the window's days by a counted range, newest first.

- Rejected: leaving `open_lapse` as a disclosed residual, because a mutant of its counter never
  ends, and one test that reached it would cost a shard its whole budget as the silence walk did.
- Rejected: converting it without the smallest-day guard, because that answers a lapse at the
  smallest epoch day where the earlier loop answered none (A41 is red under it).

## Amendment, 2026-09-30: the relight's celebration is due in the grant's own write

The amendment of 2026-09-29 answered a relight day as due in memory, inside the fold's per-day
write and before that write committed. A day could then be celebrated for a grant that rolled
back, and a day held in memory was lost at a restart between the commit and the route. The order
was chosen by a TLA+ model of the per-day write, the due list, the cycle's take and route, the
router's once-ever key and a restart between any two steps, checked for three properties: at most
one celebration per relight day (S1), none without a committed grant (S2), and every committed
grant celebrated under fair recompute (L1) (SPEC-076 R27 restated, R28; the proof is #477).

- Chosen: the day is due in the grant's own write, because only this order kept S1, S2 and L1
  with a failure or a restart between any two steps. The cycle reads the list after the fold
  commits and clears a day once the router has decided it, and the router's once-ever key still
  answers a day routed twice.
- Rejected: answering the day as due after the write commits, held in memory, because a restart
  between the commit and the route loses the day, so L1 fails.
- Rejected: clearing the due list when a fold fails, because it also drops a day an earlier write
  committed, so L1 fails even with no restart.
- Rejected: deriving the due days from the ledger's relight grants, because an imported history's
  grants would be celebrated (SPEC-140 R5), and the router would need a read of what it decided.
- Rejected: one stored slot for the due day, because a second relight day overwrites the first
  before its route, so L1 fails.
- Rejected: routing inside the fold, because a send would then leave from a write that can still
  roll back, and the one-router census (SPEC-041) names the cycle as the router's caller.
- Rejected: a window of due days bounded by a cursor, because it drops an older committed day the
  cursor has already passed.

## Amendment, 2026-09-30: the walks' domain is stated, and their proof is #478

The silence walk steps at most `SILENCE_WALK_CAP_DAYS + 1` days below today, and the law bridge
and the law replay one day. For a today that close to the smallest epoch day, the subtraction
leaves the day type. No caller passes such a day: a study day is an instant's day, and its epoch
day number lies within 2^37 of the epoch (SPEC-076 R32).

- Chosen: the domain is stated, because no caller can pass a day outside it, so a guard would
  answer at days no one reaches. Each function's doc and R32 state it, and the proof is #478.
- Rejected: guarding each walk as `open_lapse` is guarded, because that invents an answer the
  predecessor never gave: its date arithmetic raises at its smallest date. Each guard would also
  be new mutation surface, pinned only by a test at days no caller reaches.

## Amendment, 2026-09-30: the calendar is a follow-up

SPEC-076 R20, R23 and section 4 promised a calendar with freeze, skip and break markers, which
this delivery does not serve or draw (SPEC-076 section 18).

- Chosen: the calendar is excluded and follows as #486, because none of #81 to #84 lists it in its
  acceptance, and adding a served surface now would reopen the served-pairs rule (R31) mid-review.
- Rejected: delivering the calendar in this delivery, because it adds a route body, markers and a
  screen section that no acceptance of #81 to #84 asks for.
- Rejected: moving #81 from `Closes` to `Refs`, because its acceptance list is met; the calendar
  appears only in its prose.

## Amendment, 2026-09-30: a failed route leaves the day due

The relight-order amendment leaves a day due when its route fails (SPEC-076 R27 restated), but
its model had no failing route, and no test made the router answer an error. The model now has
one: the router's route of a taken day fails, before its claim is written or after the claim
committed and the line was sent, and the cycle goes on. S1, S2 and L1 were checked again with that
step, and the rejected order below, a failed route that clears the day, was checked as a witness
and violates L1 (SPEC-076 section 22, A51, A52; the proof is #477).

- Chosen: a failed route leaves the day on the list, because only a decided day may leave it.
  The cycle goes on to the next due day, the next cycle routes the failed one again, and the
  router's once-ever key answers a day it already claimed as already sent. This is the cycle's
  order at this amendment; A51 and A52 now decide it.
- Rejected: clearing the day when its route fails, because a day whose claim was never written
  is then never celebrated, so L1 fails.
- Rejected: ending the cycle's route at the first failed route, because one day whose route keeps
  failing would hold back every later day's celebration, while going on routes each day alone.
- Rejected: returning the failed day to a list held in memory for a retry, because the stored
  list already holds the day, and a list held in memory is lost at a restart.
- Rejected: retrying the failed route within the same cycle, because the next cycle's route is
  already that retry, and a loop in the cycle would add a bound and a wait that the stored list
  makes unneeded.
