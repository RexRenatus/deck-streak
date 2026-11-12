# SPEC-115: the daily digest gains a coaching paragraph that quotes only its own stats and degrades plainly

- **Wave:** W6. **Issue:** #53 (epic #7). **Context(s):** `deck-streak-agent` (the coaching task
  and its number check); `deck-streak-coordination` (the coaching step between the digest's stats
  and its send, and the degraded form).
- **Decided by:** ADR-043 (the shell runner, its gate and caps), ADR-054 (no-AI mode is the default;
  with the route absent the digest carries no coaching line and no unavailable line), ADR-113 (a
  prepared duty's message goes through the router) and ADR-115 (coaching that quotes a number its
  stats do not hold is dropped, and the digest goes out degraded).
- **Prerequisites:** SPEC-041, SPEC-043 and SPEC-101 (planned, W5; #129). **Mutation band:**
  `S11500-S11599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-115.md` (ADR-016).

## 1. The problem, measured

- **The digest exists before this SPEC.** SPEC-101 (planned, W5; #129) builds the daily digest's
  deterministic stats, its stats input and its send through the router. #53 adds the AI coaching
  paragraph, and says the digest still goes out, saying coaching was unavailable, when coaching
  fails closed. The number SPEC-101 carries is W5's planned one.
- **Nothing to port.** The parity row `SB-U21` (SPEC-001 §7, digests and nudges carrying
  second-brain data) is a build row, not a port: the predecessor's product has no coaching-paragraph
  function at `27ee2bc`, so the step, its check and its degraded form are DeckStreak's own.
- **The pack's contract.** nudge-duties' coaching rules give the step exactly two inputs, the rules
  and the stats input, never the journal; two to four sentences; a number only if the stats input
  holds exactly that number; no date, no urgency, no shame, no near miss, no internals; and "write
  nothing" when the rules cannot be followed. Its message contract gives a digest `coaching: ok`
  with a coaching part, or `coaching: unavailable` with a notice part and no coaching part
  (`digest-degraded`). Quoting an unheld number is its advisory `coaching-numbers` class only.
- **A gap between the pack and ADR-054.** With the route absent, ADR-054 says the digest carries no
  coaching line and no unavailable line, because nothing failed. The message contract has no value
  for that state, so the absent form is proved by a public test and is not a member of the pack's
  golden population (§3a).
- **This delivery lifts the nudge-duties pack's deferral**, which names #53.

## 2. Requirements

R1. The coaching step runs inside SPEC-101's digest use case, after its stats input is written and
    before its message is routed. It is `coordination::digest_coaching::coach`, the one call SPEC-101's
    digest makes; that call is this SPEC's only change to SPEC-101's files, and the delivery names
    them in a manifest amendment once they are on dev.
R2. With the route absent, the step returns at once, before any launch, and the digest is SPEC-101's
    deterministic form: its stats part only, with no coaching part and no notice part. Nothing is
    alerted (SPEC-043 R16).
R3. The task `digest-coaching` (format `other`) passes exactly two inputs: `agent/prompts/system/
    coaching-rules.md` (nudge-duties' coaching rules, copied; source `rules`) and the stats input
    (source `stats`). The journal, the vault, cards and memory are never inputs. It holds no tool;
    its caps are 120 seconds, 3 turns and SPEC-043 R6's default budget.
R4. The coaching is accepted only when the ai-content-safety pack's required classes for an `other`
    task pass AND every numeral in the coaching (a run of digits, with an optional `.` decimal part
    and an optional `%`) equals a number the stats input holds, as the stats part renders it (R7).
R5. An accepted coaching makes the digest `coaching: ok`: the stats part, then the coaching part.
    Any other outcome of the step (withheld, unavailable, a cap, or an unheld numeral) makes it
    `coaching: unavailable`: the stats part, then the notice part from nudge-duties' daily-digest
    template, and no coaching part. The cause is recorded in `agent_runs`; one alert is raised as
    SPEC-043 R12 says. The digest is sent in every case.
R6. The whole message is checked before the router sends it by nudge-duties' blocking classes
    (`message-contract`, `telegram-length`, `telegram-entities`, `digest-numbers`,
    `digest-degraded`, `no-internals`, `no-dates-or-countdowns`, `no-shame-framing` and
    `no-journal-in-messages`) and ai-content-safety's output classes, run as SPEC-043 R11 runs a
    gate. A message with an accepted coaching that fails one is sent in its degraded form instead.
R7. The numbers the check of R4 compares are the stats input's values rendered as the stats part
    renders them (`.` as the decimal mark, no grouping), so `92.5` in the input admits `92.5` and
    `92.5%` and no other spelling.
R8. The step stores nothing of its own: the coaching text lives only in the message SPEC-101
    records, and `agent_runs` holds the run's verdict and cause.
R9. `privacy.json` declares the coaching's processing by route: only with an AI route configured is
    the stats input sent to the model provider, and it holds no card or note text.
R10. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no
    dishonest copy (R4, R5), no fabricated near miss (the coaching rules' near-miss rule) and no
    forward date.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the route absent the digest is the stats part only, with no coaching and no notice, and nothing is launched or alerted | `with_no_ai_route_the_digest_is_its_stats_alone` |
| A2 | the task receives exactly the rules and the stats input | `the_coaching_receives_only_its_rules_and_stats` |
| A3 | an accepted coaching gives `coaching: ok` with the coaching part after the stats | `an_accepted_coaching_is_sent_after_the_stats` |
| A4 | a coaching quoting a numeral the stats input does not hold is dropped and the digest goes out degraded | `a_coaching_with_an_unheld_number_is_dropped` |
| A5 | `92.5` in the input admits `92.5` and `92.5%`, and not `92.50` or `93` | `the_number_check_reads_the_rendered_stats` |
| A6 | a time cap reached by the scripted runner sends the degraded digest, with its notice, and one alert | `a_coaching_that_times_out_degrades_the_digest` |
| A7 | a withheld coaching sends the degraded digest | `a_withheld_coaching_degrades_the_digest` |
| A8 | a message failing a nudge-duties class with its coaching is sent in its degraded form | `a_message_failing_its_gate_is_sent_degraded` |
| A9 | the step stores no coaching outside the digest's own record | `the_coaching_step_stores_nothing` |

```acceptance
A1: cargo test -p deck-streak-coordination --test digest_coaching -- --exact with_no_ai_route_the_digest_is_its_stats_alone
A2: cargo test -p deck-streak-coordination --test digest_coaching -- --exact the_coaching_receives_only_its_rules_and_stats
A3: cargo test -p deck-streak-coordination --test digest_coaching -- --exact an_accepted_coaching_is_sent_after_the_stats
A4: cargo test -p deck-streak-agent --test digest_coaching -- --exact a_coaching_with_an_unheld_number_is_dropped
A5: cargo test -p deck-streak-agent --test digest_coaching -- --exact the_number_check_reads_the_rendered_stats
A6: cargo test -p deck-streak-coordination --test digest_coaching -- --exact a_coaching_that_times_out_degrades_the_digest
A7: cargo test -p deck-streak-coordination --test digest_coaching -- --exact a_withheld_coaching_degrades_the_digest
A8: cargo test -p deck-streak-coordination --test digest_coaching -- --exact a_message_failing_its_gate_is_sent_degraded
A9: cargo test -p deck-streak-coordination --test digest_coaching -- --exact the_coaching_step_stores_nothing
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. This delivery lifts the nudge-duties pack's deferral:
the private wiring changes it from pending to enforced when the delivery merges, and the builder
hands back that JSON diff. ai-content-safety is enforced by its own feature (#29) before it.

| id | criterion | decided by |
|---|---|---|
| B1 | over the two golden digests under `agent/golden/daily-digest/` (one `coaching: ok`, one `coaching: unavailable`) and their stats inputs: every stat line matches its input | the nudge-duties pack (`digest-numbers`) |
| B2 | over the same two golden digests: the degraded one says so plainly with no coaching part, the other has a coaching part and no notice | the nudge-duties pack (`digest-degraded`) |
| B3 | over the same two golden digests: the message contract, length, entities, internals, dates, shame framing and journal rows, and the advisory coaching-number row reported | the nudge-duties pack's other classes |
| B4 | over `ai-safety.json`: the task declares its two inputs, no tool, its golden outputs and a complete gate | the ai-content-safety pack |

The route-absent digest (A1) is not in B1 to B3's population: the message contract has no coaching
value for it.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/agent/src/digest_coaching.rs` | `deck-streak-agent` | added: the task and the number check |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module |
| `crates/agent/tests/digest_coaching.rs` | `deck-streak-agent` | added: A4, A5 |
| `agent/prompts/system/coaching-rules.md` | agent (public) | added: nudge-duties' coaching rules, copied |
| `agent/prompts/digest-coaching.prompt.md` | agent (public) | added: the prompt with its fenced stats slot |
| `agent/golden/daily-digest/` | agent (public) | added: the two synthetic golden digests and their stats inputs |
| `agent/redteam/` | agent (public) | changed: a case in the stats input |
| `ai-safety.json` | repo | changed: the task |
| `crates/coordination/src/digest_coaching.rs` | `deck-streak-coordination` | added: `coach` and the degraded form |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/digest_coaching.rs` | `deck-streak-coordination` | added: A1 to A3, A6 to A9 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the step's ports |
| `privacy.json`, `PRIVACY.md` | repo | changed: the coaching's processing by route |
| `scripts/mutation-rows.d/S11500-S11599.json` | repo | added: the rows of §9 |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-115-the-daily-digest-gains-a-coaching-paragraph-that-quotes-only-its-own-stats-and-degrades-plainly.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w6-duty-run-and-its-degradation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-115.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It changes no stat, no send time and no layout of the digest (#129).
- It coaches no weekly report and no session debrief (#130, #131).
- It adds no coaching to the comeback message (#123).
- It builds no settings screen for coaching (#57).
- It adds no table (#53).

## 6. Risks

- **A plausible number the model computed.** Dropped by R4 before the pack's advisory row would only
  report it; detected by A4 and A5.
- **A slow coaching step delaying the digest.** A 120-second wall clock (R3), then the degraded form;
  detected by A6.
- **The absent form judged as a failure.** Kept out of the pack's population (§3a) and proved by A1.

## 7. Parity goldens

None: the stats' goldens are SPEC-101's, and the coaching text is a model's, which no golden can
fix. The two golden digests under `agent/golden/daily-digest/` are DeckStreak's own synthetic
examples for the box run.

## 8. Tables and the v9 import

None: the step adds no table (R8).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11501-ROUTE-FIRST` | `crates/coordination/src/digest_coaching.rs` | the absent route returns before any launch | `digest_coaching::with_no_ai_route_the_digest_is_its_stats_alone` |
| `S11502-NUMBER-HELD` | `crates/agent/src/digest_coaching.rs` | an unheld numeral drops the coaching | `digest_coaching::a_coaching_with_an_unheld_number_is_dropped` |
| `S11503-RENDERED-SPELLING` | `crates/agent/src/digest_coaching.rs` | the comparison uses the rendered spelling; the test names `92.5` and `92.50` | `digest_coaching::the_number_check_reads_the_rendered_stats` |
| `S11504-DEGRADE-ON-CAP` | `crates/coordination/src/digest_coaching.rs` | a cap degrades, never blocks, the digest; the scripted runner stops at the 120-second cap on a virtual clock | `digest_coaching::a_coaching_that_times_out_degrades_the_digest` |
| `S11505-DEGRADE-ON-WITHHELD` | `crates/coordination/src/digest_coaching.rs` | a withheld coaching degrades | `digest_coaching::a_withheld_coaching_degrades_the_digest` |
| `S11506-GATE-THEN-DEGRADE` | `crates/coordination/src/digest_coaching.rs` | a gate failure sends the degraded form | `digest_coaching::a_message_failing_its_gate_is_sent_degraded` |
| `S11507-INPUTS-TWO` | `crates/coordination/src/digest_coaching.rs` | only the rules and the stats reach the task | `digest_coaching::the_coaching_receives_only_its_rules_and_stats` |
