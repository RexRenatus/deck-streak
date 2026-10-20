# SPEC-368: the privacy page says exactly what DeckStreak trains or fits on your data

- **Issue:** `#722`. `PRIVACY.md` closes with one sentence about training, and it sits, by
  position, inside the section on what an erase does not reach. It is an absolute claim with no
  heading and no test, and the app does hold things a reader could call fitting or training: the
  scheduling parameters, the card memory state and two AI duties. This SPEC replaces the sentence
  with a section that says what each of them does today, and adds tests that hold the section and
  fail when a fitting call, an AI tool or an embedding computation appears.
- **Context(s):** `docs` (`PRIVACY.md`, `scripts/tests/test_privacy_policy.py`) and the records
  section 4 names. No Rust, web or iOS source changes.
- **Decided by:** ADR-379 (this SPEC's own). It amends no record: no earlier line of any SPEC or
  ADR changes.
- **Schematic:** none. No component, state, edge or store is added or removed.
- **Status:** one delivery. **Mutation band:** none (section 8). **Model:** none (section 7).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `b0935c75` by `git grep -n`, `git grep -c` and
`git show`. Nothing was run: every figure is a read.

### 1a. The sentence, and what pins it

`PRIVACY.md` ends with the line "DeckStreak never uses your data to train a model." (`:120`). It
follows the last bullet of `## What an erase does not reach` (`:91-118`), so a reader and the
file's own section parser both place it inside that section. A `git grep` for `train a model`,
`to train`, `training data`, `fine-tun` and `never uses your data` finds one hit, that line: no
test pins it.

### 1b. What could be called fitting or training

| # | candidate | what it does | fits or trains? | evidence |
|---|---|---|---|---|
| 1 | scheduling parameters | read from the collection; the Anki engine applies them when you answer | no | `crates/engine-core/src/table.rs:113` `ORDINARY: [Ordinary; 18]` and `:246` `EXEMPT: [Exempt; 6]`: 24 `Service.Method` names, none a fitting method; no hit in the tree for `compute_parameters`, `ComputeParameters`, `ComputeFsrsParams`, `compute_params`, `optimal_retention`, `evaluate_params`, `ExportDataset` or `export_dataset` |
| 2 | card memory state | `crates/ingest/src/memory_state.rs` reads the state the collection stores | no | the file's reader and its default decay |
| 3 | habit strength | a fixed half-life constant | no | `crates/streaks/src/constants.rs:25` |
| 4 | the review-memory crate | replays a synthetic history at pinned defaults; the users are examples, the measure workflow and tests | no | `crates/fsrs7/src/measure/history.rs:1-2`; `Cargo.toml:141`, `:145` |
| 5 | the two AI duties | `daily-reading-law` and `daily-reading-language`: inputs persona, duty rules, memory and cards; `tools: []`; `on_invalid: withhold` | context for one run | `ai-safety.json` tasks 0 and 1; `crates/agent/src/route.rs` (`AiRoute::Absent` is the default); `crates/agent/src/memory.rs` (leeches, graded practice and lapses, never the journal); `crates/agent/src/duty.rs:96-103`, `:175-190`; `PRIVACY.md:23` (`agent-runs` keeps "never the prompt or the reply") |
| 6 | research instruments | one, read-only, writes nothing | no | `crates/insights/src/lib.rs:3-7`, `registry.rs:27` |
| 7 | the inbox curator | planned only (`docs/specs/planned/SPEC-116-*`), not built | not at `dev` | `PRIVACY.md:46`, `privacy.json` `inbox-captures` |
| 8 | query-planner statistics | `PRAGMA optimize` | no | `crates/coordination/src/maintenance.rs:52` |

### 1c. Embeddings and similarity

No code computes an embedding or similarity data from a learner's data. The word appears in two
comments and one test list: `web/app/src/lib/card/policy.js:6` and
`web/app/src/lib/csp.test.ts:92` say "embedding" for HTML frame embedding, and
`scripts/tests/test_declared_write_classes.py:82-83` hold the strings "embeddings" and "similarity
edges" in a list of declared write classes. That is 4 lines in 3 files.

## 2. Requirements

R1. **A section, in the sentence's place.** `PRIVACY.md` gains `## Models and your data` where
    `:120` sits, and the sentence is removed. The heading ends `## What an erase does not reach`.

R2. **The paragraph.** Under the heading, exactly this paragraph (whitespace folded):

    > DeckStreak does not train, fine-tune or fit a model on your data, and does not build a
    > dataset from it. The scheduling parameters in your collection come from your Anki app:
    > DeckStreak reads them, and the memory state Anki stores for each card, to schedule the cards
    > you study, and does not fit them to your reviews. An AI duty runs only when the host's AI
    > route is configured, which it is not by default. Each run sends the cards that duty covers
    > and a summary of your leeches, lapses and graded practice, not your journal, as the context
    > of that one run, and DeckStreak's database keeps neither the prompt nor the reply. A reply
    > that passes its checks is written to your own vault. What the model's provider keeps is set
    > by that provider's terms.

R3. **Present behaviour only.** The section states what DeckStreak does today. It makes no promise
    about what DeckStreak will or will not do later, and so does not exclude a future fit of a
    learner's scheduling parameters to that learner's own reviews. Such a change owes an amendment
    of this section in its own delivery (R7).

R4. **The provider sentence names no provider.** R2's last sentence, "What the model's provider
    keeps is set by that provider's terms.", is new to the page and names no provider.

R5. **The disclosure.** A changelog fragment `changelog.d/models-and-your-data-368.md` carries a
    `### Changed` bullet that says the page now states what DeckStreak does with the data that
    could be called training. `privacy.json` does not change: no category, store or purpose
    changes, and its keys are closed. README, the about page and the bot's commands link the file,
    not the sentence, and do not change.

R6. **Three censuses, each with a planted control.**
    - The engine allow-list holds no fitting method: of its 24 methods none holds `Params`,
      `Fsrs`, `Retention`, `Simulate`, `Benchmark` or `Dataset`.
    - No engine crate names a fitting call: the eight names of section 1b row 1 appear in no
      source file of a workspace crate that depends on the Anki engine or on either review-memory
      crate.
    - No AI task holds a tool: every task in `ai-safety.json` has `tools: []`, and `agent-runs`
      still says "never the prompt or the reply".

R7. **The embedding census, and the message.** No file under `crates/`, `web/` or `ios/` computes
    an embedding or similarity data, judged by the words `embedding`, `similarity` and `cosine`,
    case-insensitively, with three exemptions held by exact path: the two frame-embedding comments,
    each exempt by its path and its exact line text, and the declared-class list, exempt by its
    path. Every exemption must match at least one line, so a stale one fails. The red message of
    each census in R6 and R7 names the amendment a delivery owes: the section `## Models and your
    data` of `PRIVACY.md`, and its changelog entry.

## 3. Acceptance criteria

The base is the red commit: the five tests below, over the unchanged `PRIVACY.md`. No stubs are
needed: the tests are text tests. No test pins an engine-computed literal.

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | `PRIVACY.md` holds the R2 paragraph under `## Models and your data`, and the old sentence is absent | `assertIn` fails: the paragraph and the heading are absent, and the old sentence is present | `scripts/tests/test_privacy_policy.py` `test_the_policy_states_what_deckstreak_trains_and_fits` (added) |
| A2 | The engine allow-list holds no fitting method: it prints `examined 24 engine methods`, refuses zero, and a planted list holding `SchedulerService.ComputeFsrsParams` is refused by that name first, with the R7 message | not red: the base's allow-list is clean, so this is a guard whose plant proves it can fail | `test_privacy_policy.py` `test_the_engine_allow_list_holds_no_fitting_method` (added) |
| A3 | No engine crate names a fitting call: it prints `examined N files in M crates`, refuses zero, and a planted line is refused first | not red: the base holds no such call, so this is a guard whose plant proves it can fail | `test_privacy_policy.py` `test_no_engine_crate_names_a_fitting_call` (added) |
| A4 | Every `ai-safety.json` task has `tools: []` (examined 2 tasks), `agent-runs` still says `never the prompt or the reply`, and a planted task with a tool is refused first | not red: the base's two tasks hold no tool, so this is a guard whose plant proves it can fail | `test_privacy_policy.py` `test_no_ai_task_holds_a_tool` (added) |
| A5 | No file in the population computes an embedding or similarity data: it prints `examined N files`, the count of exempt lines matched, and the count of drill-named paths skipped, refuses zero, and a planted line is refused first | not red: the base holds only the exempt lines, so this is a guard whose plant proves it can fail | `test_privacy_policy.py` `test_no_code_computes_an_embedding_or_similarity` (added) |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_states_what_deckstreak_trains_and_fits
A2: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_engine_allow_list_holds_no_fitting_method
A3: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_no_engine_crate_names_a_fitting_call
A4: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_no_ai_task_holds_a_tool
A5: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_no_code_computes_an_embedding_or_similarity
```

The red-first record is `docs/red-first/SPEC-368.md`, one fence line per fact: `red at <sha>:
<failure>` and `green at <sha>` for A1, and `not red: <why>` for A2 to A5.

The population of A3, measured under `crates/`: the workspace crates whose manifest depends on
`anki`, `fsrs7` or `fsrs6` are `engine-core`, `ffi`, `fsrs7`, `ingest` and `readings`. The web engine's
manifest path is not measured here: the build lists the crates it finds, records the count, and
names any crate beyond these five in its report.

A5's population is the tracked files under `crates/`, `web/` and `ios/` with the extensions `.rs`,
`.js`, `.ts`, `.svelte` and `.swift`, plus the declared-class list's file. It skips every path
whose name holds `drill` (case-insensitive) and prints how many it skipped, and skips the one
parked path `crates/vault/src/readings_tree.rs` by exact path and prints that count too, because
that surface stays parked (`#158`). The two comment lines' text is read by the build from the two
cited lines and copied into the exempt list; a cited line that has moved stops the step.

Shipped tests this delivery leaves as they are: every other test in `test_privacy_policy.py`
keeps its name and its text. The policy parser reads the bullets of `## What an erase does not
reach` only, and the new heading ends that section.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-368-the-privacy-page-says-exactly-what-deckstreak-trains-or-fits-on-your-data.md` | docs | added |
| `docs/decisions/ADR-379-the-privacy-page-states-what-deckstreak-does-with-your-data-today-and-censuses-hold-it.md` | docs | added |
| `docs/red-first/SPEC-368.md` | docs | added |
| `changelog.d/models-and-your-data-368.md` | docs | added |
| `PRIVACY.md` | docs | the closing sentence (`:120`) replaced by the section of R1 and R2 |
| `scripts/tests/test_privacy_policy.py` | tests | one class holding A1 to A5 and their plants, added before `if __name__` |

## 5. What this does NOT do

- It adds no fit of any kind, and promises none: the section states today's behaviour, and a later
  delivery that fits a learner's scheduling parameters owes the amendment R3 and R7 name
  (`#722`).
- It gives the web and iOS apps no export or erase path (`#721`).
- It changes no category, store or purpose, so `privacy.json` is unchanged (`#722`).
- It names no model provider and states nothing about what a provider keeps beyond that provider's
  own terms (`#722`).
- It builds nothing for the planned inbox curator, and the page does not describe it
  (`#722`).
- It changes no README, about page or bot command text (`#722`).
- It checks the drill surface for nothing: A5 skips drill-named paths and the parked
  `readings_tree.rs` while that surface is parked (`#158`).

## 6. Risks

- **A sentence that stops being true.** The paragraph is true of `dev` row by row (section 1b).
  A2 to A5 fail when a fitting call, an AI tool or an embedding computation lands, and their
  messages name the amendment owed. They cannot see a fit written under a name the eight words do
  not hold; the paragraph's claim then rests on review.
- **The parser reads the new section as a policy bullet.** The section holds a paragraph, not a
  bullet, and the existing class judges the bullets of `## What an erase does not reach` only. The
  build runs the whole file.
- **A5 skips drill-named paths and the parked `readings_tree.rs`.** A computation placed in one
  is not seen while that surface is parked. The skip count is printed so the blind spot is never
  silent.
- **An exempt line moves.** The two comment lines are exempt by text, not by number, so an edit
  that rewrites one makes its exemption stale and the test fails until the text is updated.
- **Frame embedding is not model embedding.** The exemption is by exact path and text, so a new
  use of the word elsewhere is read by a person and either exempted with a reason or amended.

## 7. Formal model

None. No Rust source, state machine or covered span changes.

## 8. Mutation rows

None, and no band is declared. No Rust or Python production code changes: the only code is the
test module, whose guards each carry a planted control that must be refused first.

## 9. What only CI proves

| # | what | where |
|---|---|---|
| C1 | The whole Python test suite passes with the new class and the edited page | CI on the pull request |
| C2 | A3 and A5 judge the CI checkout's tracked files, and the counts they print match the build's | CI on the pull request |
| C3 | The repository's artifact bar and public-text scan read the SPEC, ADR, fragment and page clean | CI on the pull request |

## 10. Amendments

### SPEC-387: R2's paragraph is replaced

SPEC-387 R10 replaces R2's paragraph with the text quoted there, in its own delivery, as R3
and R7 require. R1 and R3 to R7 stand, and every census of R6 and R7 is unchanged. DeckStreak
still fits nothing: a fit, when one exists, is made by the learner's Anki app, and DeckStreak
now proposes the scheduler's defaults for one preset on request and records the proposal.
