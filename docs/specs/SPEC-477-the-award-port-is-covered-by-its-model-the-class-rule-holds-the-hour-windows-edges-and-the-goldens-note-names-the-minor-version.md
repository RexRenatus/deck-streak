# SPEC-477: the award port is covered by its model, the class rule holds the hour windows' edges, and the goldens' note names the minor version

- **Issue:** #544 (the follow-up of the review of SPEC-073's part B). **Context(s):**
  `deck-streak-coordination` (the class rule of the badge step); `deck-streak-progression` (the
  award port and the hour windows, read and covered, not changed); `formal/tla/AwardOnce/` (the
  award-once model); the parity oracle under `tools/parity-oracle/` (part B).
- **Decided by:** ADR-494. Amends ADR-303 insert-only (its confirmation gains the cover).
- **Prerequisites:** SPEC-073 (the award port, the badge step, the hour windows, the award-once
  model and the goldens). **Mutation band:** `S47700-S47799`.
- **Status:** part A (the cover, the window-edge members and their rows) is delivered by its pull
  request, which references #544. Part B (the note's cure and the regeneration of every golden) is
  cut after part A lands, once no open pull request changes a golden and every committed golden can
  be regenerated in one run from the predecessor's recorded commit; its pull request closes #544.

## 1. The problem, measured

Each figure below was read from the tree this SPEC was written against.

- `git grep -n '@phx covers' -- formal | grep -c 'crates/progression/src/badges/award.rs'` reads
  0. `formal/tla/AwardOnce/AwardOnce.tla` carries twelve covers (its lines 2 to 13): the badge
  step's `evaluate`, the offers' `offer_badges`, which writes the mark, and the router's `route`
  among them. Its `Commit` action abstracts the award port's unique key as `UniqueKey`, yet a change
  to `award` (`crates/progression/src/badges/award.rs`, lines 105 to 142) is reported by nothing.
- The class rule, `every_evaluated_day_awards_exactly_the_badges_its_context_earns_once` in
  `crates/coordination/tests/badges_steps.rs`, places every night answer at hour 1 and every early
  answer at hour 5 (`git grep -n 'add(at(D0, [15])' -- crates/coordination/tests/badges_steps.rs`
  reads lines 650 and 653), under the default study-day rule only (its support's `run_step`). Its
  population varies each window's count around 50 and no member's hour: no member sits at an edge
  of the night window (from midnight up to, not including, the rollover hour) or of the early window
  (from the rollover hour up to, not including, hour 7), and none is judged under another offset or
  rollover. Read from its source, it judges 92 members, 368 evaluated days.
- `tools/parity-oracle/generate.py` line 261 writes the generating interpreter's full version into
  every adapter golden's note (`cases drawn under <implementation> <version>.`). Of the adapter
  goldens counted, 108 name one patch release and 1 names another, while all of them record one
  generator digest.
  Every golden also records the generator's sha256 (`generate.py` line 251), and
  `tools/parity-oracle/test_goldens.py` refuses any committed golden whose recorded digest differs
  from the committed generator, so any edit to the generator moves every one of the 176 goldens.

## 2. Requirements

Part A:

R1. `formal/tla/AwardOnce/AwardOnce.tla` covers `crates/progression/src/badges/award.rs` at
    `award`, with the digest of that item at the delivery's head, so a later change to `award` reads
    `STALE` on `AwardOnce`, `CelebrateAtMostOnce` and `NoSilentLoss`.
R2. The model's comments re-read `award` against `Commit`: a refused key writes nothing; one insert
    that writes nothing on a conflict of the key and tier is `UniqueKey`; a catalog badge is written
    unmarked; a band badge is written marked by a caller no cover of the model names. No constant,
    variable, action, property, witness or configuration of the model changes.
R3. The class rule's section holds a second population, the window-edge members: each is one of
    three study-day rules (the default, which the test asserts is rollover 4 at offset 0; rollover 4
    at UTC+1; rollover 5 at UTC-5:30) and one of six edges (midnight, before and after; the
    rollover on the next date, before; the rollover on the evaluated date, after; hour 7, before
    and after), with 50 answers placed one second apart on that side of the edge, from the edge's
    own millisecond, all on the evaluated study day.
R4. Each member is judged by both evaluations, twice each, with the class rule's assertions (the
    awarded set, each row's tier, day and mark, and the celebrations raised once). Its expected set
    is the class rule's own `earns` of the profile a literal table makes it equivalent to: 50
    answers in the night window, in the early window, or in neither. The test asserts that
    `night_owl` and `early_bird` are each earned by a member and missed by another, and reports and
    pins `examined 72 evaluated day(s)`.
R5. The class rule's own population, test and reported figure are unchanged: the edit to
    `badges_steps.rs` only adds lines.
R6. Rows S47700 to S47705 mutate `crates/coordination/src/progression/badge_context.rs` (the night
    window's start and end, the early window's start and end, and the rule each window reads), and
    each is killed by A1's test.

Part B:

R7. `generate.py` writes an adapter golden's note with the interpreter's implementation and minor
    version only.
R8. A regeneration under another patch release of the same minor version writes byte-equal goldens,
    and one under another minor version writes a note naming that minor version.
R9. Every committed golden is regenerated in one run from the predecessor's recorded commit, and
    each changes only in its `generator_sha256` line and, for an adapter golden, its note line.
R10. Rows S47710 (the note names the full version again) and S47711 (the note drops the minor
     version) are each killed by A2's test.

## 3. Acceptance criteria of SPEC-477

| id | criterion | decided by |
|---|---|---|
| A1 | a day of 50 answers at each edge of the night and early windows, under each of three study-day rules, awards exactly the badges its answers' window earns, once; examined 72 evaluated days | `every_window_edge_day_awards_exactly_the_badges_its_local_hours_earn_once` |

```acceptance
A1: cargo test -p deck-streak-coordination --test badges_steps -- --exact every_window_edge_day_awards_exactly_the_badges_its_local_hours_earn_once
```

## 3a. What the formal check judges

The formal check of `tla/AwardOnce` judges these at the delivery's head; the builder runs it and the
lander runs it again. They have no line in the fence, which holds test commands only.

| id | criterion | decided by |
|---|---|---|
| F1 | the entry covers `award.rs` at `award` with the item's digest, reads no `STALE`, catches every witness and reads every property clean in its configuration | the formal check of `tla/AwardOnce` at the head |
| F2 | a change to one line of `award`, committed in a scratch clone of the head, reads `STALE` naming `award.rs` on each of the entry's three properties, and the same change at the base reads no such line | the formal check of `tla/AwardOnce` in a scratch clone, at the head and at the base |

## 3c. Delivered by the next pull request

Part B delivers these. When it is built, its fence lines move into the acceptance fence above,
verbatim, and this table's rows move into section 3's table without the last column.

| id | criterion | delivered by |
|---|---|---|
| A2 | the stand-in's adapter golden, generated under two patch releases of one minor version, is byte-equal, and under another minor version names that minor version | part B |
| A3 | an adapter golden's note names the implementation and the minor version only | part B |
| A4 | every committed golden is current: its recorded generator digest equals the committed generator's | part B |

```text
A2: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_regeneration_under_another_patch_release_writes_the_same_bytes
A3: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_an_adapter_golden_names_the_function_it_drives
A4: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k test_every_committed_golden_is_current_and_well_formed
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/coordination/tests/badges_steps.rs` | A | changed, lines added only: the window-edge members, their placement, their rule-aware step run and A1's test |
| `formal/tla/AwardOnce/AwardOnce.tla` | A | changed, lines added only: the cover on `award.rs` at `award` and its re-read (R1, R2) |
| `scripts/mutation-rows.d/S47700-S47799.json` | A | added: rows S47700 to S47705; part B appends S47710 and S47711 |
| `docs/specs/SPEC-477-the-award-port-is-covered-by-its-model-the-class-rule-holds-the-hour-windows-edges-and-the-goldens-note-names-the-minor-version.md` | A | added: this SPEC; part B moves section 3c's rows and lines into section 3 |
| `docs/decisions/ADR-494-the-award-port-joins-the-award-once-model-the-window-edges-are-class-members-and-the-note-cure-waits-for-every-golden.md` | A | added: the decision |
| `docs/decisions/ADR-303-an-award-carries-its-celebration-mark-and-is-offered-until-the-router-answers.md` | A | changed, appended only: the cover on the award port |
| `docs/schematics/badges-records-and-the-next-milestone.md` | A | changed, appended only: section 5, the award path's cover, the window's edges and the goldens' note |
| `docs/red-first/SPEC-477.md` | A | added: A1's line and the planted copies' reads; part B appends A2 to A4 |
| `changelog.d/badges-award-gaps-477.md` | A | added: part A's fragment |
| `tools/parity-oracle/generate.py` | B | changed: the note's interpreter names the minor version (line 261) and its comment |
| `tools/parity-oracle/test_generate.py` | B | changed: the note's pin names the minor version, and A2's test is added |
| `tools/parity-oracle/goldens/` | B | changed: every golden regenerated, each in its digest line and an adapter golden in its note line |
| `changelog.d/goldens-note-minor-version-477.md` | B | added: part B's fragment |

## 5. What this does NOT cover

- The badge catalog, the study conditions, the records and the next milestone beyond the three gaps:
  SPEC-073's, under #74, #75 and #76.
- The goldens' cases: part B regenerates them byte-equal and changes none (#544).
- The award-once model's states, actions and properties beyond the cover and its re-read: the model
  of #74, #75 and #76 is unchanged.
- The band badges' path into the model: written marked and celebrated by the band-up (#74, #75,
  #76).
- Part B until its cut condition holds: #544 stays open until part B lands.
- Anything only a later CI run measures, such as the mutation legs' selection of S47700 to S47705,
  whose target file part A does not change: the rows are proved by hand at the head, and CI's
  reading is the lander's (#544).

## 6. Risks

- The kernel assigns a study day under an offset other than as R3 places the answers, so a member's
  answers fall outside the evaluated day: A1 then fails at the head on that member's label, and the
  build reads the kernel's rule before writing the members.
- The badge step drops a day's answers after the evaluation's clock: every member's answers are
  placed at or before it, and A1's message names the member and the evaluation.
- A planted copy of `hours.rs` is also killed by SPEC-073's golden of the hour windows, so its read
  in the red-first record names A1's assertion, never only a failure.
- Part B cannot check out the predecessor's recorded commit, or runs under another minor version:
  its regeneration would change cases, and it stops before any golden is written.
