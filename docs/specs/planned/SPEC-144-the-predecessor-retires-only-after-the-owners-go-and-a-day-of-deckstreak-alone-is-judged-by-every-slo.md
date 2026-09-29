# SPEC-144: the predecessor retires only after the owner's go, and a day of DeckStreak alone is judged by every SLO

- **Wave:** W8. **Issue:** #63 (epic #9), both criteria: the owner's go is recorded before the
  stop, and a day of DeckStreak alone passes every SLO. The owner gate #164 decides the go.
  **Context(s):** `deck-streak-coordination` (the `retired` and `alone` steps of the cutover
  ledger), `deck-streak-daemon` (the two commands, and the import's apply held until the
  retirement), `deploy` (the SLO evaluator's day report and the runbook).
- **Decided by:** ADR-011 (the predecessor is stopped and retired only by the owner's explicit go,
  and kept disabled until the owner approves its removal), ADR-031 (an SLO sized for one owner),
  ADR-143 (the cutover ledger's steps) and ADR-144 (the day alone is judged by a report over one
  full day after the import, recorded by its digest, and a day with no traffic proves nothing).
- **Prerequisites:** SPEC-027 and SPEC-031 (landed); SPEC-142, SPEC-143 and SPEC-146 (this wave,
  unlanded: the import role, the cutover ledger, and the stats job's item, without which the
  retirement could pass with the stats contract unmoved). **Mutation band:** `S14400-S14499`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-144.md` (ADR-016).

## 1. The problem, measured

- **Nothing orders the stop after the go.** SPEC-143 records the go and each contract's move, and
  SPEC-142 applies the import from a copy of the predecessor's database. Nothing records that the
  predecessor stopped as a whole, so nothing refuses an apply over a copy the predecessor could
  still be changing, or a retirement before the go.
- **The SLO evaluator pages; it does not judge a day.** At `dev` 703097b
  `deploy/scripts/slo-evaluate.py` reads the API's response events and prints a burn episode at
  priority 3 when both windows of an alert burn (SPEC-031 R4). It has no mode that answers "did
  this day meet every objective", and `deploy/slo.json` declares one SLO, `api-availability`, at
  objective 0.99 over 28 days, with `low_traffic` `longer-window` (ADR-031).
- **A quiet day would pass.** An evaluator that finds no response in a window does not burn, so a
  day in which nothing reached the API would read as met.

## 2. Requirements

R1. `deckstreakd cutover retired --reference <ref>` records the step `retired` (SPEC-143 R5): the
    private rail (#41) has stopped the predecessor as a whole and keeps it disabled. It is refused
    with `no_go` before the go, with `items_open` while any `moves` item's last step is not
    `verified`, and with `retired` once recorded. One `Db::write` transaction reads the ledger and
    appends the row.
R2. `deckstreakd import --apply` (SPEC-142 R6) is refused with `not_retired`, before the copy is
    read, unless the ledger holds `retired`. The dry run is not held: it rehearses on a copy of
    DeckStreak's database and writes nothing live (SPEC-142 R5).
R3. `deploy/scripts/slo-evaluate.py DECLARATION --day-report --since <s> --until <u>` judges every
    SLO of the declaration over `[s, u)`, from the same response events the burn alerts read. It
    prints `since=<s>` and `until=<u>`, then one line per SLO,
    `slo=<id> good=<n> total=<n> objective=<o> verdict=<pass|fail|void>`, and ends
    `SLO DAY PASS`, `SLO DAY FAIL: <n>` or `SLO DAY VOID: <n>`. An SLO is `void` when its total is
    0 or its journal cannot be read, `pass` when good over total, as an exact fraction, is at
    least its objective, and `fail` otherwise. The day is `PASS` only when every SLO passes; a
    `fail` outranks a `void`. Exit codes: 0 pass, 1 fail, 3 void, 2 usage. The mode writes no
    episode record and prints no priority-3 line, so it pages no one.
R4. `deckstreakd cutover alone --report <file>` records the step `alone` with the command line and
    the report's sha256. It is refused unless `retired` is recorded (`not_retired`), unless the
    report's last line is `SLO DAY PASS` (`not_passed`), unless `until - since` is at least 86,400
    seconds (`not_a_day`), and unless `since` is at or after the `retired` row's time
    (`before_retirement`). The report stays in the maintainer's private record; only its digest
    enters the ledger.
R5. The runbook `deploy/cutover.md` gains `## Retirement` and `## The day alone`, in this order
    after the last item of SPEC-143's checklist: the rail's retirement, recorded with `retired`
    (#41, #164); SPEC-142's runbook `deploy/v9-import.md` from its final copy to its apply; the day
    report over the first full day after the apply; and `alone`. It states that the predecessor
    stays disabled, not removed, until the owner approves its removal (#164), and that a failed or
    void day is the owner's decision, with SPEC-142's rollback and SPEC-143's revert as the ways
    back.
R6. Every refusal names its reason and writes nothing. The eleven anti-goals hold (CHARTER): no
    step grants XP, sends a message or deletes a file, and every step is run by hand.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a retirement recorded before the go is refused with `no_go` and writes nothing | `a_retirement_before_the_go_is_refused` |
| A2 | a retirement while one moving item is only switched is refused with `items_open`, naming the item | `a_retirement_with_an_item_not_verified_is_refused` |
| A3 | the import's apply before the retirement is refused with `not_retired` and reads nothing, and its dry run is not held | `an_apply_before_the_retirement_is_refused` |
| A4 | a day report passes only when every SLO's good over total meets its objective, one SLO one response short fails, and a fail outranks a void | `test_a_day_passes_only_when_every_slo_meets_its_objective` |
| A5 | a day with no response event is void and exits 3 | `test_a_day_with_no_response_is_void` |
| A6 | the day report writes no episode record and prints no priority-3 line | `test_the_day_report_pages_nothing_and_keeps_no_state` |
| A7 | a day alone is refused before the retirement, for a report that did not pass, for a span under a day, and for a day that began before the retirement | `an_alone_day_needs_a_passing_full_day_after_the_retirement` |
| A8 | a recorded day alone holds the command line and the report's sha256, and no line of the report | `the_alone_record_holds_the_reports_digest` |
| A9 | the runbook orders the retirement, the import and the day alone after the checklist, and every step after the go names #164 | `test_the_runbook_orders_retirement_import_and_the_day_alone` |
| A10 | the runbook names no removal of the predecessor | `test_the_runbook_keeps_the_predecessor_disabled_not_removed` |

```acceptance
A1: cargo test -p deck-streak-coordination --test retirement -- --exact a_retirement_before_the_go_is_refused
A2: cargo test -p deck-streak-coordination --test retirement -- --exact a_retirement_with_an_item_not_verified_is_refused
A3: cargo test -p deck-streak-daemon --test cutover -- --exact an_apply_before_the_retirement_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_slo_evaluator.py -k test_a_day_passes_only_when_every_slo_meets_its_objective
A5: python3 -m unittest discover -s scripts/tests -p test_slo_evaluator.py -k test_a_day_with_no_response_is_void
A6: python3 -m unittest discover -s scripts/tests -p test_slo_evaluator.py -k test_the_day_report_pages_nothing_and_keeps_no_state
A7: cargo test -p deck-streak-coordination --test retirement -- --exact an_alone_day_needs_a_passing_full_day_after_the_retirement
A8: cargo test -p deck-streak-daemon --test cutover -- --exact the_alone_record_holds_the_reports_digest
A9: python3 -m unittest discover -s scripts/tests -p test_cutover_runbook.py -k test_the_runbook_orders_retirement_import_and_the_day_alone
A10: python3 -m unittest discover -s scripts/tests -p test_cutover_runbook.py -k test_the_runbook_keeps_the_predecessor_disabled_not_removed
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges this over the committed tree and posts its
verdict on the pull request as the `box/packs` status. It has no line in the acceptance fence. The
observability pack is already enforced, so this delivery changes no pack's state and hands back no
wiring change.

| id | criterion | decided by |
|---|---|---|
| B1 | every SLO row of the observability pack passes over `deploy/slo.json` and `deploy/scripts/slo-evaluate.py`, examining the one declared SLO, `api-availability`, and the evaluator's day-report mode beside its burn alerts | the observability pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/cutover.rs` | `deck-streak-coordination` | changed: the `retired` and `alone` steps (R1, R4) |
| `crates/coordination/tests/retirement.rs` | `deck-streak-coordination` | added: A1, A2, A7 |
| `crates/daemon/src/role_cutover.rs` | `deck-streak-daemon` | changed: `retired` and `alone` |
| `crates/daemon/src/role_import.rs` | `deck-streak-daemon` | changed: the apply held until the retirement (R2) |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | changed: the usage line names the two commands |
| `crates/daemon/tests/cutover.rs` | `deck-streak-daemon` | changed: A3, A8 |
| `crates/daemon/tests/import.rs` | `deck-streak-daemon` | changed: a fixture that runs the role's apply records the go and the retirement first |
| `deploy/scripts/slo-evaluate.py` | deploy | changed: `--day-report` (R3) |
| `scripts/tests/test_slo_evaluator.py` | repo | changed: A4-A6, in a new class `TheDayReport` |
| `deploy/cutover.md` | deploy | changed: `## Retirement` and `## The day alone` (R5) |
| `scripts/tests/test_cutover_runbook.py` | repo | added: A9, A10 |
| `.sqlx/` | repo | changed: the refreshed query cache |
| `scripts/mutation-rows.d/S14400-S14499.json` | repo | added: §9's rows |
| `docs/schematics/cutover-checklist-and-sequence.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-144.md` | docs | added: the red-first record |
| `docs/specs/SPEC-144-the-predecessor-retires-only-after-the-owners-go-and-a-day-of-deckstreak-alone-is-judged-by-every-slo.md` | docs | moved from `docs/specs/planned/` |
| `changelog.d/feat-cutover-day-alone.md` | repo | added: the fragment |

## 5. What this does NOT do

- It stops, disables and removes nothing of the predecessor: the private rail carries out the
  stop on the owner's go and records it with `retired` (#41, #164).
- It removes nothing: the predecessor stays disabled until the owner approves its removal (#164).
- It moves no contract; the checklist does, one at a time (#62).
- It declares no new SLO and changes no objective: the declaration is SPEC-031's (#24).
- It tags no release: v1.0.0 follows the day alone (#64).

## 6. Risks

- **An import over a copy the predecessor was still changing.** Prevented by R2's hold until the
  retirement, and detected by A3 and row S14403.
- **A retirement before the go, or with a contract still unproved.** Prevented by R1's refusals,
  and detected by A1, A2 and rows S14401 and S14402.
- **A quiet day read as a pass.** Prevented by R3's `void`, and detected by A5 and row S14405.
- **A near-miss read as a pass**, from a float or a rounding. Prevented by R3's exact fraction, and
  detected by A4 and row S14404.
- **A day report that pages.** Prevented by R3's stateless mode, and detected by A6 and row S14406.
- **A day judged before the predecessor stopped.** Prevented by R4's `before_retirement`, and
  detected by A7 and row S14407.

## 7. Parity goldens

None: the retirement and the day alone are DeckStreak's own steps, with no predecessor function to
prove.

## 8. Tables and the v9 import

No table. The two steps are rows of SPEC-143's `cutover_steps`.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14401-GO-FIRST` | `crates/coordination/src/cutover.rs` | the retirement waits for the go | `retirement::a_retirement_before_the_go_is_refused` |
| `S14402-ITEMS-VERIFIED` | `crates/coordination/src/cutover.rs` | the retirement waits for every item | `retirement::a_retirement_with_an_item_not_verified_is_refused` |
| `S14403-APPLY-HELD` | `crates/daemon/src/role_import.rs` | the apply waits for the retirement | `cutover::an_apply_before_the_retirement_is_refused` |
| `S14404-OBJECTIVE` | `deploy/scripts/slo-evaluate.py` | good over total at least the objective | `test_slo_evaluator.TheDayReport.test_a_day_passes_only_when_every_slo_meets_its_objective` |
| `S14405-VOID` | `deploy/scripts/slo-evaluate.py` | no response is void | `test_slo_evaluator.TheDayReport.test_a_day_with_no_response_is_void` |
| `S14406-NO-PAGE` | `deploy/scripts/slo-evaluate.py` | the day report pages no one | `test_slo_evaluator.TheDayReport.test_the_day_report_pages_nothing_and_keeps_no_state` |
| `S14407-AFTER-RETIREMENT` | `crates/coordination/src/cutover.rs` | the day begins after the retirement | `retirement::an_alone_day_needs_a_passing_full_day_after_the_retirement` |

No killer calls a network: the evaluator's tests feed a stub journal, and the Rust tests run
against a temporary database.
