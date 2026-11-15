---
status: "accepted"
decision-makers: "the DeckStreak architect"
---

# The award port joins the award-once model, the class rule holds the hour windows' edges, and the goldens' note names the minor version once every golden can be regenerated

## Context and Problem Statement

The review of SPEC-073's part B left three gaps (#544). None is a defect; each is a check the
repository does not keep.

1. `award` in `crates/progression/src/badges/award.rs` is named by no `@phx covers` line. The
   award-once model `formal/tla/AwardOnce/` covers twelve spans, among them the badge step's
   `evaluate`, the offers' `offer_badges` (which writes the mark) and the router's `route`, so a
   change to any of those reads `STALE` on the model's three properties. A change to `award`, whose
   unique key the model's `Commit` action abstracts as `UniqueKey`, reads nothing.
2. The class rule of `crates/coordination/tests/badges_steps.rs` (every evaluated day awards exactly
   the study badges its context earns, once) places its night answers one hour after midnight and
   its early answers one hour after the rollover, under the default study-day rule only. No member
   sits at an edge of either hour window, and none is judged under another offset or rollover, so a
   window that moved by an hour, or read its rule's offset twice or not at all, would still pass it.
3. `tools/parity-oracle/generate.py` writes, into every adapter golden, a note that names the
   generating interpreter's patch release. A regeneration under another patch release of the same
   minor version then changes the note's bytes while every case stays equal.

This ADR decides how each gap is closed and how the work is cut.

## D1. The mechanism, as measured

All facts below were read at the base this decision was written against, by file and line.

**The award.** `award` (`award.rs:105-142`) first asks `key_kind` (`award.rs:86-97`) which rule
accepts the key: a catalog key, or `band_<code>_<band>` for a configured course and a band from A1
to C2. A key no rule accepts is refused with `AwardError::UnknownKey` before any write
(`award.rs:110`). A catalog badge is written with its mark unset and a band badge with its mark set
to the award's instant (`award.rs:113-116`). The write is one `INSERT ... ON CONFLICT DO NOTHING`
into `badges_earned` (`award.rs:121-136`); the comment at `award.rs:120` states that the primary key
on the key and tier is the existence check (the migration that declares it was not read). One
written row answers `Award::Awarded`, none answers `Award::AlreadyAwarded` (`award.rs:137-141`).
Three steps call it inside the fold's write: the badge step (`recompute/badges.rs:98`), the habit
badges (`recompute/habit_badges.rs:70`) and the band badges (`recompute/band_badges.rs:66`). The
badge step treats both answers alike (`recompute/badges.rs:99`). The offers read every unmarked row
(`award.rs:149-168`), hand each to the router, and only after an answer set the mark in a write of
their own, guarded by `AND celebrated_at IS NULL` (`recompute/badges.rs:189-206`).

**The model and its covers.** `formal/tla/AwardOnce/AwardOnce.tla` carries twelve covers on lines
2 to 13, each `\* @phx covers <path> anchor=<item> digest=sha256:<64 hex>`. None names a file under
`crates/progression/`. The model's `Commit` (lines 195-212) writes nothing when `UniqueKey` holds and
the day's award exists, and its property `AwardOnce` (line 281) is one award per key; the witness
`an-award-with-no-unique-key.cfg` kills it (line 18). Its prose (lines 60-61) already names "the
progression award port's unique key" as the thing `UniqueKey` abstracts. A cover's digest is the
sha256 of the item's lines from its definition through its closing brace, each followed by a
newline: that rule reproduces the recorded digests of `badges.rs::offer_badges` and
`badges.rs::evaluate` exactly, and gives `award` the digest
`17e9e6c76d81877062c2722aff7d3e3a581e68317d753aaeb92d8b6d78e5b3c1`. The repository's CI runs no
formal step, so the cover is read by the formal check that the delivery's builder and its lander run.

**The window the class rule varies.** The badge context counts the night window from hour 0 up to,
not including, the rollover hour, and the early window from the rollover hour up to, not including,
hour 7 (`crates/coordination/src/progression/badge_context.rs:122-135`; `EARLY_BIRD_END_HOUR` at
`crates/progression/src/badges/conditions.rs:25-26`). Both call `count_in_local_hours`
(`crates/progression/src/badges/hours.rs:25-40`), which keeps a review of the evaluated study day
whose local hour lies in `start..end` (`hours.rs:32`, start inclusive, end exclusive), the local hour
being the instant shifted by the rule's offset and never by the rollover (`hours.rs:13-20`). A
window whose start is above its end is an empty range, so the code admits no window that crosses
midnight; the night window starts at it. The class rule builds every member under the default rule,
whose rollover hour is 4 at offset 0 (`crates/coordination/tests/awards_support/mod.rs:30-31`,
`:236-262`), puts night answers at hour 1 and early answers at hour 5 (`badges_steps.rs:627-660`,
shifted by a second per answer), and varies each window's count around its threshold of 50
(`badges_steps.rs:486-487`). Its reference is `earns` (`badges_steps.rs:566-624`), its own
arithmetic over literal thresholds. It reports `examined <n> evaluated day(s)` and asserts the
figure equals members times two evaluations times two passes (`badges_steps.rs:740-747`); read
from the source, the population is 92 members, so 368 days (the figure was not run). How the
kernel assigns a study day under a non-zero offset was not read.

**The note.** `golden_for` writes, for an adapter golden only, `<the registration's note>; cases
drawn under <implementation> <full version>.` (`generate.py:258-264`). Every golden, adapter or not,
also records `generator_sha256`, the sha256 of `generate.py` (`generate.py:251`), and
`test_goldens.py` refuses any committed golden whose recorded digest differs from the committed
generator (`test_goldens.py:149-167`, `:311-323`, `:428-451`), reading every `goldens/*.json`
(`test_goldens.py:125-126`). The goldens directory holds 176 goldens; of the 168 this decision read,
all record one generator digest and one predecessor commit, and 109 carry the note: 108 name one
patch release and one names another. `test_generate.py:282-283` pins the note's form. No mutation
row anchors the note's lines (24 rows anchor `generate.py`, none of them there).

## D1 was chosen against

- Reading the class rule's examined figure from a run: this round runs no test, so the figure 368 is
  the source's count, and the build re-measures it at its cut before any edit.
- Assuming the kernel's study day under an offset from the hour window's code: the reads do not show
  it, so the build reads the kernel's rule at its cut and stops if a window member would fall on
  another study day than the one the test places it on.

## D2. The tests and the cover

**The window's edges are class members of their own.** A new test in the class rule's section,
`every_window_edge_day_awards_exactly_the_badges_its_local_hours_earn_once`, judges a second
population by the same evaluations, passes and assertions as the class rule. Each member is one
study-day rule and one edge: 50 answers (the threshold of both windows) placed one second apart on
one side of the edge, starting at the edge itself when they follow it and at its last millisecond
when they precede it. The rules are the default (the test first asserts it reads rollover 4 at
offset 0), rollover 4 at UTC+1, and rollover 5 at UTC-5:30. The edges, in local time on the
evaluated day's calendar date D or the date after it, D+1, are:

| edge | side | local hour | window the member's answers lie in |
|---|---|---|---|
| midnight | before | 23 on D | none: the night window starts at midnight |
| midnight | after | 0 on D+1 | night (its start is inclusive) |
| the rollover on D+1 | before | rollover - 1 on D+1 | night (its end is exclusive) |
| the rollover on D | after | rollover on D | early, and not night |
| hour 7 | before | 6 on D | early |
| hour 7 | after | 7 on D | none (the early window's end is exclusive) |

Every member's answers lie on the evaluated study day. The reference is the class rule's own
`earns`, applied to the profile a member is equivalent to: 50 answers inside the night window, 50
inside the early window, or 50 ordinary answers, as the table's last column says. So the expected
set is decided by a literal table and the class rule's literal thresholds, never by the window's
code. The two midnight edges are the window that meets midnight: the code admits no window that
crosses it, so no member configures one. The population is three rules by six edges, 18 members, and
the test pins `examined 72 evaluated day(s)` (18 x 2 evaluations x 2 passes) as a literal, beside the
class guard that `night_owl` and `early_bird` are each earned by a member and missed by another. The
class rule's own population, code and figure are not changed: the edit to `badges_steps.rs` is
insert-only.

**What each axis must catch**, each a copy of the code planted at the base and read red by the new
test's awarded-set assertion:

| axis | planted copy | the member that sees it |
|---|---|---|
| the night window's start | the night window starts at hour 1 | midnight, after |
| the night window's end | the night window ends at the rollover, included | the rollover on D, after |
| the early window's start | the early window starts an hour after the rollover | the rollover on D, after |
| the early window's end | the early window ends at hour 7, included | hour 7, after |
| the configured rule, night | the night window reads the default rule | the two offset rules' night members |
| the configured rule, early | the early window reads the default rule | the two offset rules' early members |
| the hour range | `start..=end` in `hours.rs` | the rollover on D, and hour 7, after |
| the range's start | `start + 1..end` in `hours.rs` | midnight, after, and the rollover on D, after |
| the offset applied twice | the offset added twice in `local_hour` | rollover - 1 on D+1, and hour 6, under UTC+1 |
| the offset not applied | the offset multiplied by zero in `local_hour` | the rollover on D, under UTC+1 |
| the hour's floor | the local hour rounded to the nearest | midnight, before, and hour 7, before |

The first six are mutation rows on `badge_context.rs`, the window's code in the crate whose test
kills them. The last five are planted in `hours.rs`, whose crate cannot name another crate's test as
a row's killer; they are read red in the red-first record, and the parity golden of
`count_in_local_hours` (SPEC-073 A8) stays their crate's own check.

**The cover.** `AwardOnce.tla` gains one cover line after its last one:
`\* @phx covers crates/progression/src/badges/award.rs anchor=award digest=sha256:17e9e6c76d81877062c2722aff7d3e3a581e68317d753aaeb92d8b6d78e5b3c1`,
and a written re-read of `award` in the `Commit (award)` paragraph: a refused key writes nothing (a
stutter), one conflict-free insert on the key and tier is `UniqueKey`, a catalog badge is written
unmarked (design "mark"), and a band badge is written marked by a caller no cover of this model
names, so for the model's key the band arm is a stutter. No constant, variable, action, property,
witness or configuration changes. A later change to `award` then reads `STALE` on `AwardOnce`,
`CelebrateAtMostOnce` and `NoSilentLoss`, as a change to the mark's `offer_badges` or the router's
`route` does today.

## D2 was chosen against

- Adding the edge members to the class rule's own population: the class rule's test would gain no
  new name, so the red-first stage would read an existing test changed and none added, and its
  reported figure, 368, would move.
- Edges at the hour only: a member placed inside an hour, as the class rule's answers are (one second
  apart from hour 1 and hour 5), cannot see a local hour rounded instead of floored, and never reaches
  the edge's own millisecond.
- A member per hour of the day, 24 by 3 rules: 72 members, four times the database scratches, and
  no planted copy above is seen by an hour the six edges miss.
- A second offset only, at the default rollover: a copy that reads the default rollover where the
  configured one is meant would pass every member, so one rule moves the rollover too.
- Rows on `hours.rs` with the coordination test as their killer: a row's killer is a test of the
  row's own crate, so such a row could not be selected.
- A new formal entry for the award port alone: the property the cover must stale is `AwardOnce`,
  which already abstracts the port as `UniqueKey`, and a second model would restate it.
- A test in the repository that recomputes the cover's digest: it would be a second formal reader
  beside the formal check, which is the one reader the house keeps.

## D3. Production or test

The first part changes no production line: a test, a model's comments and cover, a rows band and
documents. The second part changes one production line, `generate.py:261`, so that the note names
the implementation and its minor version only (`<implementation> <major>.<minor>`), and the comment
above it says why. Each adapter golden's note then moves once, from its full version to its minor
version, and every golden's `generator_sha256` moves to the new generator's digest, while every case,
every other field and every byte outside those two lines stays equal. Two test changes hold it:

- `test_generate.py:282-283`, which pins the whole note by equality, pins the minor form instead: it
  is changed, not weakened, since it still compares the whole note.
- A new test in `GeneratorWritesGoldensFromTheRegistry` generates the stand-in's adapter golden
  under two patch releases of one minor version and reads the two files byte-equal, then under the
  next minor version and reads them different, with the note naming that minor version. At the base
  it is red: the two patch releases write two notes.

The regeneration itself is read by the goldens' diff: each of the 176 changes in exactly its
`generator_sha256` line, and an adapter golden in its note line too. `test_goldens.py` changes
nothing and its examined figures do not move; `test_generate.py` gains one test and moves no
figure. Two rows guard the new line (D5).

## D3 was chosen against

- Rewriting each golden's digest and note by a script, without running the generator: a golden is
  the predecessor's own output, and a provenance field written by hand would claim a regeneration
  that never ran.
- Hashing the generator without its note line: `test_goldens.py`'s staleness pin would then read
  part of the file, a weakening no delivery makes, only so that the goldens need no regeneration.
- Dropping the interpreter from the note: the note says which interpreter drew the cases, and a
  seeded sequence may change between minor versions; the next minor version must still read
  different.

## D4. The shape

One SPEC, SPEC-477, this ADR and one rows band, S47700-S47799, in two parts. Part A is the cover,
the window-edge members and their rows. Part B is the note's cure with the regeneration of every
golden. Part B is cut only after part A lands, and only when no open pull request changes a golden
and every committed golden can be regenerated in one run from the predecessor's recorded commit: the
cure's new generator digest moves every golden, two open pull requests change goldens today, and a
golden part B cannot regenerate would stay stale. Part A's pull request references #544; part B's
closes it, since #544 is done only when all three gaps are. SPEC-073 is not amended: neither part
changes a requirement or a criterion it states. ADR-303 is amended insert-only, because its
confirmation names the model and this delivery widens what the model covers. The schematic of
SPEC-073, `docs/schematics/badges-records-and-the-next-milestone.md`, gains a section insert-only.

## D4 was chosen against

- One delivery holding all three gaps: its regeneration would rewrite the goldens two open pull
  requests also change, and it could not land until every golden can be regenerated, holding the
  cover and the window's edges behind a condition they do not share.
- Three deliveries, one per gap: the cover and the window's edges share the class rule's file, its
  band and one red-first record, and a third pull request would add a cut, a review and a land for
  no file the first two do not touch.
- A new SPEC for the note's cure: it is one of #544's three gaps, and the band and the record are
  shared.

## D5. Tests, rows, the formal decision and the pushes

Part A's criterion is MUTATION COVERAGE: the window's edges already hold at the base, so its test
is green there and its reds are the planted copies of D2, six of them rows. The record says so in
one `not red` line, and the red-first stage reads a new test name in a test-only change. Its rows,
S47700 to S47705, each mutate `badge_context.rs` and are killed by the new test. Before the test
exists, each row's mutant is run against the badge step's and the badge context's tests at the base
to measure the gap: a mutant one of them already kills is recorded by name and stays a row. Part
B's criterion is red at the base, read before its push; its rows are S47710 (the note names the full version
again) and S47711 (the note drops the minor version), each killed by the new test.

Formal: part A is REQUIRED on the surface already modelled: the cover joins `tla/AwardOnce`, with no
change to its states, and the build checks the entry, reads every witness caught and every property
clean, and, in a scratch clone, reads a planted change to `award` reported `STALE`. A new cover
moves the entry's inputs, so each of its three properties' promotion streaks restarts at this
landing. Lean is not applicable to either part: the hour windows are a pure function that decides
no verdict, ledger, money or release, closed by the predecessor's golden and the class rule's
literal table, and the note is a string the byte-equal test closes.

Pushes: one for each part, since every red is read before its push. Model: opus for both. Part A
hand-proves six rows and five plants, each restored by sha256 and rebuilt before the next, and
writes a model's re-read; part B regenerates every golden from a private checkout and
reads 176 diffs by count, where a mis-step rewrites provenance.

## D5 was chosen against

- A RED-FIRST label for part A's test: it is green at the base, and a test green at the base is no
  red-first evidence.
- Two pushes for part A, the first to read a red in CI: every red is read before the push, so a
  second push would only spend a CI cycle.
- Sonnet for part A: a stop in the middle of a hand proof would leave a mutated source in the tree.

## D6. What this delivery does not do

The badge catalog, the conditions, the records and the milestone beyond the three gaps; the goldens'
cases; the award-once model's states beyond the cover; part B until its cut condition holds; and
anything only a later CI run measures. SPEC-477 lists each with its issue.

## D6 was chosen against

- Re-reading the band badges' path into the award-once model: band badges are written marked and
  celebrated by the band-up, which another model holds, and no gap of #544 names them.

## Consequences

- A change to `award` is reported by the formal check, like every other span of the award path.
- The class rule's file holds the window's edges under three rules, and six rows keep the planted
  copies killed; the class rule's own population and figure are unchanged.
- Until part B lands, a regeneration under another patch release still moves the note's bytes.

## What would make this wrong

- The kernel assigns a study day under an offset differently from the test's placement, so a
  member's answers fall on a day the reference does not count: the build reads the kernel's rule at
  its cut and stops.
- The badge step filters a day's reviews by the evaluation's clock: the members' answers are placed
  at or before it, and a stop names the first one dropped.
- The predecessor's recorded commit can no longer be checked out, or the generator's interpreter is
  of another minor version: part B cannot regenerate byte-equal cases and stops before any golden is
  written.
