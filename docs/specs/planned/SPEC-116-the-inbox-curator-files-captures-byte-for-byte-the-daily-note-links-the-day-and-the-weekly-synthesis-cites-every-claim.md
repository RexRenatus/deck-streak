# SPEC-116: the inbox curator files captures byte for byte, the daily note links the day, and the weekly synthesis cites every claim

- **Wave:** W6. **Issues:** #50 (the inbox curator) and #49 (the daily note and the weekly synthesis)
  (epic #7). **Context(s):** `deck-streak-vault` (the inbox snapshot, the filing moves, the periodic
  notes' names and the daily note's composition); `deck-streak-agent` (the curator's and the
  synthesis' tasks and their acceptance); `deck-streak-coordination` (one use case per duty, and the
  `inbox_filed` occasion).
- **Decided by:** ADR-042 (the vault's write paths, note format and rails), ADR-043 (the shell runner, its
  gate and caps), ADR-054 (no-AI mode is the default), ADR-113 (a prepared duty's ping is a router
  occasion) and ADR-116 (each vault duty is its own staged run, the curator runs first, and a
  journal-kind capture is never an input).
- **Prerequisites:** SPEC-020, SPEC-041, SPEC-042, SPEC-043, SPEC-110 and SPEC-118. **Mutation
  band:** `S11600-S11699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-116.md` (ADR-016).

## 1. The problem, measured

- **The adapter exists; the duties do not.** SPEC-042 executes a staged run: only `create`, `update`
  and `move`, never a delete, only inside the duty's folders from `crates/vault/data/layout.json`,
  never over an existing note, and only after the vault-duties pack's blocking classes are green; a
  red class discards the run (SPEC-042 R4). The layout gives the curator `moves_from` the inbox and
  `moves_to` its destinations, the daily note `@daily` and the synthesis `@weekly`.
- **The pack's contracts.** The daily note has three sections, `readings`, `drills` and `inbox`,
  none empty, each linking what the engine prepared for it (`daily-note`). The synthesis has
  `themes`, `connections` and `questions`, declares its `sources`, and every claim cites a source
  that was one of its inputs (`synthesis-cites`). A filed capture keeps its name and its bytes, whose
  hash is the inbox snapshot's (`never-deletes`, `note-names`). The journal is never an input and
  never quoted (`journal-never-leaks`).
- **The daily note needs no model.** Its template is headings and links the engine prepares, so it
  is written with the route absent too. The curator and the synthesis need a model.
- **The daily note is tomorrow's** (#49), and it links the notes of the day that is ending, which
  exist when the pass runs, so its links resolve after the run (`note-links`). The curator runs
  first, so the note can link what it filed.
- **The vendored layout carries no periodic format.** The pack's public layout names `YYYY-MM-DD`
  and `GGGG-[W]WW`; the owner's layout, with the real folders and the journal, is private and is
  passed to each run by path.
- **This delivery lifts the vault-duties pack's deferral**, which names #49.

## 2. Requirements

The inbox curator (#50)

R1. The inbox snapshot lists every capture in the layout's inbox that SPEC-118 recorded with state
    `captured`: its stub, its attachment (if any), each file's SHA-256 and its kind. A capture of
    kind `journal` is left out of the snapshot, so it is never an input and never moved (ADR-116).
R2. The task `inbox-curator` (format `other`) passes exactly: the layout's destinations (source
    `config`), and for each snapshot capture its id, kind, attachment name and stub caption (source
    `vault`). No attachment's bytes and no other note are inputs. It holds no tool; its caps are 240
    seconds, 5 turns and SPEC-043 R6's default budget.
R3. Its output is a filing plan: for each capture id, one destination or `leave`. The engine builds
    the staged run from it: a `move` of the stub and of its attachment, each under its own name, to
    the destination. A plan is refused whole when it names an id outside the snapshot, a
    destination outside the layout's `moves_to`, or one capture twice.
R4. Before each move the executor reads the source's bytes again; a source whose SHA-256 is not the
    snapshot's, or that is no longer in the inbox, refuses the run whole with `capture_changed` or
    `capture_moved`, and nothing is moved.
R5. A filed capture's `inbox_captures` row (SPEC-118) records its destination and the study day, in
    the same write that follows the run. The curator never updates or deletes a stub's text.
R6. When a run files at least one capture, the occasion `inbox_filed` is raised through the router,
    naming the count and no file name. The policy gains the kind as SPEC-041 R10 added
    `reading_ready`: class `nudge`, tiers `["T2"]`, budget `null`, dedupe `per-study-day`, setting
    `inbox_filed_enabled`, and a `deviations` entry naming ADR-113.

The layout and the daily note (#49)

R7. The layout in force is SPEC-118 R4's: the owner's, by `DECKSTREAK_VAULT_LAYOUT`, else the
    vendored default. The engine writes its path into each run record's `layout`, which SPEC-042
    R4's executor reads. The vendored default gains the two periodic formats of the vault-duties
    pack's public layout, `YYYY-MM-DD` daily and `GGGG-[W]WW` weekly, so a layout that names no
    format names these.
R8. The daily note is TOMORROW's: the periodic note of the study day after the one the pass runs
    in, named by the layout's daily format in its daily folder. The engine composes it from
    `agent/duties/daily-note.template.md` (vault-duties' template, copied), with no model:
    `readings` links the pass's study day's readings (SPEC-042 R5), `drills` links the active
    unanswered drills (SPEC-110), and `inbox` links the captures filed on the pass's study day (R5),
    in that order of sections. An empty section holds the line "Nothing new.". Every link carries a
    date-free alias, so the note's name is the one place a date stands.
R9. The daily note is created once: a study day whose daily note already exists (the owner's or an
    earlier run's) is skipped with `daily_note_exists`, and nothing is written.

The weekly synthesis (#49)

R10. On a pass whose study day is a Sunday, the synthesis of that ISO week is created once, named by
    the layout's weekly format in its weekly folder; an existing one is skipped with
    `synthesis_exists`.
R11. Its inputs, as a set, are the week's notes: the readings of the week's study days, the drills
    graded in the week (SPEC-110's `drill_grades`), and the captures filed in the week, newest
    first, at most 30 notes, each read up to 4000 characters (paged; source `vault`), plus
    `agent/duties/weekly-synthesis.template.md` (source `template`). A note under a folder the
    layout names in `journal`, or a capture of kind `journal`, is never an input.
R12. The task `weekly-synthesis` (format `other`) holds no tool; its caps are 300 seconds, 10 turns
    and SPEC-043 R6's default budget. The engine accepts its output only when `sources` is a subset
    of the inputs, every `themes` and `connections` claim links at least one source, every
    connection links at least two, every source is cited, and the three sections are present;
    then the executor's classes run on the staged run (SPEC-042 R4).

All three

R13. A red class, a refused plan or a withheld verdict discards the run: nothing in the vault
    changes, and SPEC-043 R12's verdict and its one alert are recorded. One duty's discard does not
    stop the next.
R14. With the route absent, the curator and the synthesis record `ai_route_absent` and write
    nothing, and the daily note is written as R8 says.
R15. `privacy.json` declares the processing by route: only with an AI route configured is a
    capture's caption, or a note's text of the week, sent to the model provider. No journal text is
    ever sent.
R16. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no dishonest
    copy (a discarded run writes nothing), no unbounded notification volume (one `inbox_filed` per
    study day at most) and no forward date in public (the vault is the owner's, and R8's aliases keep
    every date out of a note's body).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the snapshot lists each captured stub and attachment with its hash, and leaves a journal-kind capture out | `the_snapshot_leaves_journal_captures_out` |
| A2 | the curator's task receives exactly the destinations and each snapshot capture's id, kind, attachment name and caption | `the_curator_receives_exactly_its_inputs` |
| A3 | a plan naming an unknown id, a destination outside `moves_to`, or one capture twice is refused whole | `a_plan_off_its_bounds_is_refused_whole` |
| A4 | a capture whose bytes changed after the snapshot refuses the run and nothing moves | `a_changed_capture_refuses_the_run` |
| A5 | a capture already moved out of the inbox refuses the run and nothing moves | `a_capture_moved_twice_is_refused` |
| A6 | a filed capture keeps its name and its bytes, and its row records the destination | `a_filed_capture_keeps_its_name_and_bytes` |
| A7 | a run that files a capture raises `inbox_filed` naming the count | `a_filing_run_raises_inbox_filed` |
| A8 | tomorrow's daily note links the day's readings, the unanswered drills and the day's filed captures, each with a date-free alias | `the_daily_note_links_the_day` |
| A16 | each run record names the layout in force by its path, and a layout with no periodic format names the pack's defaults | `the_run_record_names_the_layout_in_force` |
| A9 | an empty section holds its line, never nothing | `an_empty_daily_section_says_nothing_new` |
| A10 | an existing daily note is skipped and nothing is written | `an_existing_daily_note_is_left_alone` |
| A11 | the synthesis runs on a Sunday only, once per week | `the_synthesis_runs_once_on_sunday` |
| A12 | the synthesis inputs are the week's notes, at most 30, never a journal note or a journal capture | `the_synthesis_never_reads_the_journal` |
| A13 | an uncited claim, a one-note connection or a source outside the inputs is withheld | `an_uncited_synthesis_is_withheld` |
| A14 | a red class from the executor discards the run and leaves the vault byte for byte | `a_red_gate_leaves_the_vault_untouched` |
| A15 | with the route absent the curator and the synthesis write nothing and the daily note is written | `with_no_ai_route_only_the_daily_note_is_written` |

```acceptance
A1: cargo test -p deck-streak-vault --test inbox_snapshot -- --exact the_snapshot_leaves_journal_captures_out
A2: cargo test -p deck-streak-coordination --test vault_duties -- --exact the_curator_receives_exactly_its_inputs
A3: cargo test -p deck-streak-agent --test vault_duties -- --exact a_plan_off_its_bounds_is_refused_whole
A4: cargo test -p deck-streak-vault --test inbox_snapshot -- --exact a_changed_capture_refuses_the_run
A5: cargo test -p deck-streak-vault --test inbox_snapshot -- --exact a_capture_moved_twice_is_refused
A6: cargo test -p deck-streak-coordination --test vault_duties -- --exact a_filed_capture_keeps_its_name_and_bytes
A7: cargo test -p deck-streak-coordination --test vault_duties -- --exact a_filing_run_raises_inbox_filed
A8: cargo test -p deck-streak-vault --test periodic -- --exact the_daily_note_links_the_day
A9: cargo test -p deck-streak-vault --test periodic -- --exact an_empty_daily_section_says_nothing_new
A10: cargo test -p deck-streak-coordination --test vault_duties -- --exact an_existing_daily_note_is_left_alone
A11: cargo test -p deck-streak-coordination --test vault_duties -- --exact the_synthesis_runs_once_on_sunday
A12: cargo test -p deck-streak-coordination --test vault_duties -- --exact the_synthesis_never_reads_the_journal
A13: cargo test -p deck-streak-agent --test vault_duties -- --exact an_uncited_synthesis_is_withheld
A14: cargo test -p deck-streak-coordination --test vault_duties -- --exact a_red_gate_leaves_the_vault_untouched
A15: cargo test -p deck-streak-coordination --test vault_duties -- --exact with_no_ai_route_only_the_daily_note_is_written
A16: cargo test -p deck-streak-coordination --test vault_duties -- --exact the_run_record_names_the_layout_in_force
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. This delivery lifts the vault-duties pack's deferral:
the private wiring changes it from pending to enforced when the delivery merges, and the builder
hands back that JSON diff. From then on, SPEC-111's staged-run row is judged too.

| id | criterion | decided by |
|---|---|---|
| B1 | over the golden runs under `agent/golden/vault-duties/` (a curator run, a daily note, a synthesis) and `crates/vault/data/layout.json`: every blocking class, `never-deletes`, `write-confinement`, `note-names` and `no-dates` among them | the vault-duties pack |
| B2 | over the golden daily note run: its three sections, each linking what its run names | the vault-duties pack (`daily-note`) |
| B3 | over the golden synthesis run: every claim cites a source among its inputs | the vault-duties pack (`synthesis-cites`) |
| B4 | over the same golden runs, with the synthetic vault fixture that holds a journal note: no input from and no quote of it | the vault-duties pack (`journal-never-leaks`) |
| B5 | over `ai-safety.json`: both tasks declare their fenced `vault` inputs, no tool and a complete gate | the ai-content-safety pack |
| B6 | over `notifications-policy.json`: `inbox_filed` is a nudge with a setting and a dedupe | the notifications-policy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/vault/src/inbox_snapshot.rs` | `deck-streak-vault` | added: the snapshot and the filing moves' checks |
| `crates/vault/src/periodic.rs` | `deck-streak-vault` | added: the periodic notes' names and the daily note's composition |
| `crates/vault/src/capture_store.rs` | `deck-streak-vault` | changed: a filing's destination and study day |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the modules |
| `crates/vault/tests/inbox_snapshot.rs` | `deck-streak-vault` | added: A1, A4, A5 |
| `crates/vault/tests/periodic.rs` | `deck-streak-vault` | added: A8, A9 |
| `crates/vault/tests/fixtures/vault-duties/` | `deck-streak-vault` | added: a synthetic vault with an inbox, readings, drills and a journal note |
| `crates/agent/src/vault_duties.rs` | `deck-streak-agent` | added: the curator's plan and the synthesis' acceptance |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module |
| `crates/agent/tests/vault_duties.rs` | `deck-streak-agent` | added: A3, A13 |
| `agent/duties/daily-note.template.md`, `agent/duties/weekly-synthesis.template.md`, `agent/duties/inbox-curator.run.template.json` | agent (public) | added: vault-duties' templates, copied |
| `agent/prompts/inbox-curator.prompt.md`, `agent/prompts/weekly-synthesis.prompt.md` | agent (public) | added |
| `agent/golden/vault-duties/` | agent (public) | added: synthetic golden runs |
| `agent/redteam/` | agent (public) | changed: a case in a caption and in a note of the week |
| `ai-safety.json` | repo | changed: both tasks |
| `crates/coordination/src/vault_duties.rs` | `deck-streak-coordination` | added: `curate`, `daily_note` and `weekly_synthesis` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/vault_duties.rs` | `deck-streak-coordination` | added: A2, A6, A7, A10 to A12, A14 to A16 |
| `notifications-policy.json` | repo | changed: the kind `inbox_filed` |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the use cases' ports |
| `crates/vault/data/layout.json` | `deck-streak-vault` | changed: the two periodic formats of the pack's public layout |
| `privacy.json`, `PRIVACY.md` | repo | changed: the curator's and the synthesis' processing by route |
| `scripts/mutation-rows.d/S11600-S11699.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-116-the-inbox-curator-files-captures-byte-for-byte-the-daily-note-links-the-day-and-the-weekly-synthesis-cites-every-claim.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/inbox-capture-and-curation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-116.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It runs no duty on a schedule or on request: the vault pass and `/vaultops` do (#54, #155).
- It writes no capture into the inbox (#154, #56).
- It never moves, reads or files a journal-kind capture; the owner files it (#56).
- It updates no note the owner wrote, and never an existing daily note or synthesis (#49).
- It builds no settings screen for the note formats (#57).
- It imports nothing from the predecessor (#61).

## 6. Risks

- **A capture lost in a move.** The executor never deletes, refuses a changed or moved source, and
  never overwrites (R4, SPEC-042 R4); detected by A4 to A6.
- **The journal reaching a model.** Left out of the snapshot and the synthesis' inputs (R1, R11);
  detected by A1 and A12, and on the box by B4.
- **A synthesis that invents its sources.** Refused by R12; detected by A13.

## 7. Parity goldens

None: the three duties are built (SPEC-001 §7, `SB-U11`, `SB-U13`, `SB-U17`), not ported, and the
filing of a capture is byte for byte. The golden runs under `agent/golden/vault-duties/` are
DeckStreak's own synthetic examples for the box run.

## 8. Tables and the v9 import

None: the filing's record is a column of SPEC-118's `inbox_captures` (R5).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11601-JOURNAL-OUT` | `crates/vault/src/inbox_snapshot.rs` | a journal-kind capture never enters the snapshot | `inbox_snapshot::the_snapshot_leaves_journal_captures_out` |
| `S11602-HASH-CHECK` | `crates/vault/src/inbox_snapshot.rs` | a changed source refuses the run | `inbox_snapshot::a_changed_capture_refuses_the_run` |
| `S11603-MOVED-CHECK` | `crates/vault/src/inbox_snapshot.rs` | a source gone from the inbox refuses the run | `inbox_snapshot::a_capture_moved_twice_is_refused` |
| `S11604-PLAN-DESTINATION` | `crates/agent/src/vault_duties.rs` | a destination is in `moves_to` | `vault_duties::a_plan_off_its_bounds_is_refused_whole` |
| `S11605-PLAN-ONCE` | `crates/agent/src/vault_duties.rs` | one capture once per plan | `vault_duties::a_plan_off_its_bounds_is_refused_whole` |
| `S11606-EMPTY-SECTION` | `crates/vault/src/periodic.rs` | an empty section holds its line | `periodic::an_empty_daily_section_says_nothing_new` |
| `S11607-DAILY-ONCE` | `crates/coordination/src/vault_duties.rs` | an existing daily note is skipped | `vault_duties::an_existing_daily_note_is_left_alone` |
| `S11608-SUNDAY` | `crates/coordination/src/vault_duties.rs` | the synthesis runs on a Sunday; the test names Saturday and Sunday | `vault_duties::the_synthesis_runs_once_on_sunday` |
| `S11609-SOURCES-CAP` | `crates/coordination/src/vault_duties.rs` | at most 30 notes; the test holds 31 | `vault_duties::the_synthesis_never_reads_the_journal` |
| `S11610-CLAIM-CITES` | `crates/agent/src/vault_duties.rs` | an uncited claim is withheld | `vault_duties::an_uncited_synthesis_is_withheld` |
| `S11611-CONNECTION-TWO` | `crates/agent/src/vault_duties.rs` | a connection links two notes | `vault_duties::an_uncited_synthesis_is_withheld` |
| `S11612-RED-DISCARDS` | `crates/coordination/src/vault_duties.rs` | a red class writes nothing | `vault_duties::a_red_gate_leaves_the_vault_untouched` |
| `S11613-ROUTE-FIRST` | `crates/coordination/src/vault_duties.rs` | the model duties check the route first | `vault_duties::with_no_ai_route_only_the_daily_note_is_written` |
