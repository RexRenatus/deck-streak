---
status: accepted
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat, ruling on the design pass for #127's second part"
---

# A landmark is offered between the fold's writes from a cursor only the router's answers move

## Context and Problem Statement

SPEC-102 R5 and R6 were written before ADR-303. They raise each landmark due on a day the fold
settles or evaluates from a step of the awards phase, inside the fold's write, and seed the
predecessor's high-water mark on the first run. ADR-303 keeps every celebration out of the fold's
writes: the router opens a write of its own, so a raise from inside a day's write waits out the busy
timeout, and the owed awards are offered between the writes instead (SPEC-102 section 10.5). An
award carries its own mark, the row the router's answer sets. A landmark has no row: it is computed
from the study days of the whole scoped log (R1, R2, ADR-318), so nothing records which landmarks
the router has answered. How are the landmarks offered between the writes, how does the recompute
know which of them it still owes across crashes and recomputes, and how does the first run's cap
of one hold when two recomputes race or one recompute offers several times?

## Decision Drivers

- A landmark is raised once (the router's once-ever key), and none is passed over silently: a
  landmark the router did not answer is offered again (ADR-303's "offered until the router
  answers").
- The first run raises at most one landmark, the first due today, as the predecessor's
  `run_landmarks` does, and seeds the mark as the predecessor's bytes (SPEC-102 R6, section 8).
- The covered items of the fold (`run`, `offer_owed`, `PHASES`, `register`) stay as they are, so
  the formal entries that cover them keep their digests.
- No migration, no new dependency edge, no daemon edit: every production cycle already holds a
  router (SPEC-319 R1).

## Considered Options (the alternatives it was chosen against)

| option | chosen, or why it lost |
|---|---|
| `LandmarkOffers` (coordination's `recompute/landmarks.rs`) offered after the awards by `OffersInTurn` on every offer call, built once a recompute by the sync cycle from the whole log's study days | chosen because it runs between the fold's writes as the awards do, and leaves the covered `run` and `offer_owed` as they are |
| the landmarks inside `AwardOffers` | rejected because `AwardOffers` holds no collection data, and "award" names badges, records and band-ups only |
| `FoldInput.offers` as a slice of offers | rejected because it edits `run` and `offer_owed`, which AwardOnce, BandUpOnce and MintReadsTheFinalBase cover |
| R5 read literally: a raise from a `DayStep` in the awards phase | rejected because a router call inside the fold's write waits out the busy timeout (ADR-303) |
| a `DayStep` handing the day's landmarks to after the fold, in memory | rejected because a crash between the day's settle and the offer loses them |
| the offers built in the daemon | rejected because the window and the study days a recompute reads exist only inside the sync cycle |
| the cursor `landmarks_offered_through` in notifications' `notification_settings`, moved only after the router answered, in a write of its own, never backwards | chosen because it is one value notifications already owns, exported and erased with its table, and it records the router's answers where the router keeps them |
| the cursor in analytics' rollup | rejected because it records the router's answers, which notifications owns: analytics would learn a notifications concept |
| a table of one mark per landmark | rejected because it needs a migration and the six data-rights files, which ADR-303 refused for an outbox |
| offering every landmark of the whole log on every call | rejected because it writes a decision row per landmark per call, and raises the history the predecessor's first run never raises |
| the cursor moved to the settle cursor whatever the router answered | rejected because it passes a landmark the router never answered (witness `a-cursor-moved-before-the-router-answered`) |
| the advance re-reading the settle cursor in its own write | rejected because it passes a day settled after the call read it, whose landmark the call never offered (witness `a-cursor-moved-to-a-settle-the-offer-never-read`) |
| a persisted set of the answered keys dated after the cursor | rejected because it is more state (a second key or a JSON value) for rows the router already bounds, and a second write site the model would have to carry; one `already_recorded` decision row per recompute on a landmark's day is accepted instead (SPEC-102 section 11.8) |
| the seed in one `BEGIN IMMEDIATE` write that reads the mark and inserts what is absent, the run first exactly when that read saw no mark | chosen because racing first recomputes serialize on the write, so exactly one of them reads the mark absent |
| the seed's read outside its write, the run first only when its own insert won | rejected because the loser of a seed race reads no mark and raises every due landmark (witness `a-loser-of-the-seed-race-raises-every-due-landmark`) |
| a first run whose cap holds on its first offer call only | rejected because its later calls raise every due landmark (witness `a-first-run-whose-later-offer-raises-every-due-landmark`) |
| a first run that moves the cursor | rejected because a day settled during the first run carries its unraised landmarks past the cursor (witness `a-first-run-that-moves-the-cursor-past-its-unraised-landmarks`) |
| the cursor started from the imported mark, otherwise from the settle cursor (design-571 P3), or set to the settle cursor at the first call | rejected because the mark holds two ordinals, not a day, and is written once (`landmarks.py:168-180`); a cursor at the settle cursor of the first call owes the days that recompute settles after the seed, raising a backlog on the first run |

## Decision Outcome

Chosen options: "`LandmarkOffers` after the awards", "the cursor in `notification_settings`" and
"the seed in one write", because together they offer each landmark between the fold's writes,
move the cursor past a landmark only once the router answered it, and hold the first run to one
landmark across every call and every race.

- **The offers.** `sync_cycle` builds, when its cycle holds a router, `OffersInTurn`: the awards'
  offers, then `LandmarkOffers` over the study days of the whole scoped log (ingest's read, once a
  recompute) and the language study days of the window. A failed whole-log read offers no landmark
  that recompute and moves no cursor. `OffersInTurn` runs each of its offers on every offer call,
  in turn, and logs one that fails without stopping the next.
- **The cursor.** `landmarks_offered_through` holds an epoch day as decimal text, X: every landmark
  dated after the seed and at or before X has been answered by the router. Each offer call reads X
  and the settle cursor S (`daily_rollup.settled_at`) on one read, and owes, oldest first, every
  landmark dated after X and at or before S, and every landmark due today, less the keys this
  recompute already had answered. An `Ok` from the router (sent, held or withheld, each a decision
  row) answers a key; an `Err` leaves it owed. The call then moves X, in a write of its own and
  only upward, to S, or to the day before the oldest owed landmark at or before S that the router
  did not answer. The S is the one the call read.
- **The seed.** The first offer call of a recompute opens one `BEGIN IMMEDIATE` write: it reads
  `landmark_high_water`, inserts it when absent as the predecessor's JSON
  (`{"anniversary": A, "seeded": true, "study_day": S}`), and inserts X as the day before today
  when absent. The run is first exactly when that read saw no mark. A first run owes, on every
  call, only the first landmark due today, and never moves X. A run that finds an imported mark and
  no cursor is not first: X is yesterday, and every landmark due today is raised.

### Consequences

- Good, because a landmark due on a day the fold settles is raised at that day's settle, and one
  due today at the current day's evaluation, each once (the router's once-ever key).
- Good, because a crash between the router's answer and the advance re-offers the landmark, which
  the router answers `already_recorded`; a crash before the answer leaves it owed.
- Good, because the fold's covered items and the daemon are unchanged, and no table is added.
- Bad, because today's landmarks are re-offered by every recompute that day and at the day's
  settle, each answered `already_recorded` with a decision row: bounded by the landmarks times the
  recomputes on their days (SPEC-102 section 11.8).
- Bad, because a late review that moves a study-day ordinal onto a day the cursor has passed raises
  nothing, as the predecessor never raises it (#580).
- Bad, because the landmarks dated between the predecessor's last run and the first recompute of
  this part are history, as the predecessor's first run considers only today (#127).
- Neutral, because while `celebrations_enabled` is `"0"` (#402) the router withholds each landmark
  `nudges_disabled`, which is an answer, so the cursor passes it as every award is marked.

### Confirmation

SPEC-102 A25 to A27 and A35 to A39 drive the sync cycle with the real router over a temporary
deployment; rows S10205 to S10207 and S10223 to S10228; `formal/tla/LandmarkOnce` checks
`FirstRunAtMostOne`, `CelebrateAtMostOnce` and `NoSilentLoss`, and each of its six witnesses is
caught. Its budget row in `config/formal.json` is `"tla/LandmarkOnce": 420`, admitted by the
architect seat's ruling 89. `formal check --entry tla/LandmarkOnce` read `FORMAL OK` in 244 s of
wall time on the maintainer's machine, at the commit that added the entry: three properties, each
checked over the same model config of 2,343,337 distinct states, plus the six witnesses. The row is
244 x 1.5 = 366 s, rounded up to a multiple of 60 s, so 420 s. The ruling chose this row over a
smaller model config or a lower state floor, because each narrows what the entry reaches and the
floor stays at `states >= 2343337`. It also chose it over folding the three properties into one
checker run, because the checker runs one per property, which is the tool's shape and not this
delivery's. The entry is measured again at the delivery's final head, and the row holds only while
1.5 times that measurement, rounded up to a minute, stays at or below 420 s.

## What would make this wrong

- A router answer of `Ok` that decided nothing: the cursor would pass a landmark nobody raised.
- A settle cursor that moved backwards, or a fold that settled a day after S while an earlier day
  stayed open: the owed range would read a different set of days than the offers answered.
- A second writer of `landmarks_offered_through`: the model checks this writer only.
- The predecessor's mark bytes differing from `high_water_mark`'s on any case of the golden
  `landmarks_run`: SPEC-141's import would read a mark this part did not write.

## More Information

SPEC-102 sections 2 (R5, R6), 8 and 11; ADR-303; ADR-318; ADR-319 and SPEC-319 R1 to R3; ADR-071;
`formal/tla/LandmarkOnce`; issues #127, #571, #580.
