# SPEC-381: a deck the learner keeps away from AI reaches no AI duty, and every deck starts readable

- **Issue:** #751. A per-deck setting, off by default: when it is on, no AI duty reads that deck's
  cards, and when the setting cannot be read, no AI duty reads any card. **Context(s):**
  `deck-streak-ingest` (the mark and its rule), `deck-streak-agent` (the gate in front of the
  runner), `deck-streak-readings` (the day set's selection), `deck-streak-coordination` (the use
  cases), `deck-streak-api` (the two routes), `deck-streak-daemon` (the wiring) and the web Mini App
  (`web/app`).
- **Decided by:** ADR-392 (this SPEC's own). It works under SPEC-043's duty run (R9 to R17: the AI
  route is optional and every refusal is recorded) and CHARTER rules 13, 16 and 17, and it changes
  none of them.
- **Schematic:** `docs/schematics/web-study-screens.md`, amended insert-only by a new last section,
  "AI and your decks": the mark from its store to the web setting, and to the one gate in front of
  every AI duty, with the fail-closed path.
- **Status:** one delivery. **Mutation band:** `S38100-S38199` (section 9). **Model:** one TLA+
  entry and one Lean entry (section 8).

## 1. The problem, measured

Every figure below was read at `e7ecf10d` with `git grep -n` or `git show <sha>:<path>`.

- **No deck carries a setting of DeckStreak's own.** The engine's call table admits no deck write
  (`crates/engine-core/src/table.rs:122-249`), and a deck's options preset is shared by every deck
  that uses it. Nothing in the tree records a per-deck choice about AI.
- **One runner reaches a model.** `git grep -n -E 'impl[^\n]*Runner for|runner\.run\(' e7ecf10d --
  crates` finds one production `Runner` impl, `ProcessRunner` (`crates/agent/src/runner.rs:81`),
  which launches `agent/run-headless.sh` (`:104`), and one production call of it,
  `self.runner.run(&prompt, &duty.caps)` (`crates/agent/src/duty.rs:178`), inside `decide` (`:165`).
  Before it stand the route check (`:166-168`) and the input gate (`:169-173`), and `compose` at
  `:175`. The other `runner.run(` matches are the job runner (`crates/daemon/src/role_job.rs:132`)
  and the instrument runner (`crates/coordination/src/instruments.rs:342`), which reach no model.
- **The duty does not know which deck a card came from.** `Parts` (`crates/agent/src/compose.rs:10-27`)
  carries two untrusted slots, `memory` and `cards`, as text. `DutyInput` holds a template, a subject
  and the parts, and no deck.
- **No AI duty runs in production yet.** `DutyEngine` is built only in
  `crates/agent/tests/duty.rs:56` and `crates/agent/tests/redteam.rs:91`, and the day set's
  resolution, `resolve_study_day` (`crates/coordination/src/readings/resolve.rs:90`), is called only
  from `crates/coordination/tests/readings_resolve.rs:165`. So the gate must stand where every
  later caller passes, in the duty run itself, and a census must hold each new AI call site to it.
- **The memory and tool ports carry no deck content today.** `MemoryPort`
  (`crates/agent/src/memory.rs:135`) has one production impl, `DrillGradesMemory`
  (`crates/daemon/src/wiring.rs:722-751`), which returns an exercise type and its XP. The MCP server
  exposes one tool, `get_law_track` (`crates/mcp/src/tools.rs:130-136`), which returns aggregates
  (streak, XP, level and counts).
- **A card has two decks.** The reader keeps a card under its home deck,
  `CASE WHEN odid != 0 THEN odid ELSE did END` (`crates/ingest/src/reader.rs:38-43`, `home_deck_id`
  at `:138`), and names a deck's top level by `top_level` (`:160`). A card a filtered deck borrowed
  sits in that deck now and belongs to its home deck.
- **A refusal has a place to be recorded.** `agent_runs` admits `withheld` only with a non-empty
  `class` (`migrations/004301_agent_runs.sql`), and `decide` already records the input gate's refusal
  as `withheld` through `withhold` (`duty.rs:169-173`). The verdict set is closed
  (`crates/agent/src/verdict.rs`), so this SPEC adds no verdict and no cause.

## 2. Requirements

R1. **The mark.** The server keeps the learner's marked decks in a new table, `sensitive_decks`,
owned by `ingest` and created by `migrations/038101_ingest_sensitive_decks.sql`: one row per marked
deck, keyed by the collection's deck id (`deck_id INTEGER PRIMARY KEY CHECK (deck_id > 0)`), with
`created_at`, `STRICT`. No row means the deck is not marked, so a new install, a new deck and an
erased account mark nothing.

R2. **The one rule.** `ingest::sensitive::admits` decides every card: it refuses a card when its
home deck or its current deck, or any ancestor of either by name, is marked; when either deck id is
not in the collection's deck tree; or when the marked set could not be read. It admits every other
card. Ancestry is read with the reader's own name rule (`reader.rs:160`). No other code decides
whether a deck is kept away.

R3. **The selection.** `resolve_study_day` reads the marked set before it resolves a day. A failed
read ends the resolution with a named error, records nothing and fails the job loudly, as
`ResolveError::Ledger` does. The day set (`crates/readings/src/day_set.rs`) holds back every card
`admits` refuses, through one function, `hold_back_sensitive`, and logs the number held back,
never a deck name.

R4. **The gate.** `DutyInput` carries the deck scope of its cards: each card's home and current deck
ids. `DutyEngine` holds a `DeckGate` port, and `decide` asks it after the route check and before the
input gate, `compose` and the runner. A scope the gate refuses as kept away ends the run `withheld`
with class `deck-sensitive`. A scope the gate cannot judge (the set unreadable, a deck unresolved)
ends it `withheld` with class `deck-unreadable`, and so does a run whose cards are not empty and
whose scope is. Each refusal raises the one alert `withhold` already raises, its findings carry
counts only (never a deck name, a card's text or an id), and the runner is never started.

R5. **The wiring.** `crates/daemon/src/wiring.rs` implements `DeckGate` over the ingest store and the
collection's deck tree, with `admits` as its only decision.

R6. **The census.** `crates/agent/tests/deck_gate_census.rs` reads every production source under
`crates/*/src` but the drill-named ones, which it skips by name before any open and only counts
(section 5), with comments and string literals blanked except where a shape is a literal, and
finds every site of these shapes: the agent launch script's name; an `impl … Runner for`; a
`Command::new(` process launch; an `impl … MemoryPort for`; an MCP `#[tool(`; and a production
`DutyEngine {` literal. It holds the sites it found equal to a declared list, in which each site is
`deck-gate` or `not-deck-content` with its reason, and it pins that in `decide` the gate's call comes
before `compose(` and before `self.runner.run(`. It refuses each planted fixture under
`crates/agent/tests/fixtures/census/` by name before it judges the tree, prints `examined N` and
refuses zero, prints `fenced N` for the drill-named files it skipped, and prints a site as
`path:line:shape`, never its text.

R7. **The routes.** `GET /api/decks/sensitive`, behind the owner's session, answers
`{"decks":["<id>",…]}`: the marked ids as decimal strings, ascending. `PUT
/api/decks/{id}/sensitive`, behind the owner's session and the same-origin state-change check
(`crates/api/src/session_routes.rs:182`), takes `{"sensitive":true}` or `{"sensitive":false}` and
answers the new set. An id that is not a positive decimal integer is refused with 400. An id the
server has not ingested yet is accepted, so a new deck can be kept away before its first sync
reaches the server. Both go through `crates/coordination/src/sensitive_decks.rs`.

R8. **The web setting.** A new screen, `/study/ai-decks`, linked from the deck list, shows the
engine's deck tree with one switch per deck, off unless the deck is marked. A deck under a marked
deck shows on and cannot be changed, with a note that names its marked ancestor. Turning a switch
saves at once; a failed save turns the switch back and says so; a failed load says so and offers a
retry. `ROUTES` gains the path, and each switch is a native checkbox with the `switch` role.

R9. **The words.** English: the link and the title "AI and your decks"; the lead "Turn on a deck's
switch to keep its cards away from AI features. Studying works the same either way."; the switch
"Keep {name} away from AI"; the note "Kept away because {parent} is."; "These settings could not be
loaded."; and "That change was not saved. Try again." Each of the seven locale files carries every
key, none empty.

R10. **The learner's rights.** `privacy.json` gains a category `sensitive-decks` (source
`learner-input`, exported, erased by delete), `PRIVACY.md` gains its row, and
`crates/ingest/src/data_rights.rs` exports the marks and erases them, so after an erase every deck
is readable again until the learner keeps one away (CHARTER rule 13).

R11. **Sync and older clients.** The collection sync carries no mark and the engine's table is
unchanged. The server is the only reader of the marks, so a client that does not know them
(an older web build, the native clients) cannot weaken them: it shows no setting, and the gate still
holds every marked deck.

R12. **The formal model.** The interleaving of a mark, a selection and a gate is a TLA+ entry, and
`admits` is a Lean entry (section 8).

R13. **The threat model.** The PUT route's controls (the owner's session and the state-change check)
and the gate's refusal join the web client's rows of
`docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md`,
each citing the line that holds it.

## 3. Acceptance criteria of SPEC-381

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | A fresh server marks no deck; a mark and an unmark round-trip through the two routes | the stub routes answer 501 | `no_deck_is_kept_away_until_the_learner_marks_one` |
| A2 | `admits` refuses a card whose home deck, current deck, or an ancestor of either is marked, and admits the unmarked control | the stub admits every card | `a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed` |
| A3 | `admits` refuses a card whose deck the tree cannot resolve, and a failed read of the set | the stub admits every card | `an_unresolved_deck_or_an_unreadable_set_reads_as_kept_away` |
| A4 | A duty whose scope holds a marked deck ends `withheld`, class `deck-sensitive`, and the runner never starts | the stub calls the gate and drops its verdict | `a_kept_away_deck_is_withheld_before_the_runner_starts` |
| A5 | A duty whose gate cannot read the set ends `withheld`, class `deck-unreadable`, and the runner never starts | the stub drops the verdict | `an_unreadable_gate_is_withheld_before_the_runner_starts` |
| A6 | A duty with cards and an empty scope ends `withheld`, class `deck-unreadable`, and the runner never starts | the stub admits it | `cards_with_no_deck_scope_are_withheld_before_the_runner_starts` |
| A7 | The day set holds back every card of a marked deck and keeps the rest | the stub holds back nothing | `a_kept_away_decks_cards_are_never_selected_into_a_day_set` |
| A8 | A failed read of the set ends the resolution with its named error and records no day | the stub reads a failure as an empty set | `an_unreadable_set_resolves_no_day_and_records_nothing` |
| A9 | Every AI call site is declared, the gate stands before `compose(` and the runner, and every planted site is refused by name | `decide` calls no gate | `every_ai_call_site_passes_the_deck_gate_or_carries_no_deck_content` |
| A10 | A cross-site or signed-out PUT is refused and marks nothing | the stub answers 501 | `a_cross_site_or_signed_out_change_marks_nothing` |
| A11 | The export carries the marks and an erase leaves every deck readable | the stub exports and erases no mark | `the_marks_are_exported_and_an_erase_leaves_every_deck_readable` |
| A12 | Every switch is off until the learner turns it on, and turning one on saves the mark | the stub screen saves nothing | `a deck's switch is off until it is turned on, and turning it on saves the mark` |
| A13 | A deck under a marked deck shows on, cannot be changed, and names its marked ancestor | the stub shows it off | `a deck under a kept-away deck shows on and cannot be changed` |
| A14 | A failed save turns the switch back and shows the save message | the stub leaves it on | `a failed save turns the switch back and says so` |
| A15 | Every locale holds every key of the screen, none empty | no locale holds the keys | `every locale holds every AI-and-your-decks key` |

```acceptance
A1: cargo test -p deck-streak-api --test sensitive_decks -- --exact no_deck_is_kept_away_until_the_learner_marks_one
A2: cargo test -p deck-streak-ingest --test sensitive -- --exact a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed
A3: cargo test -p deck-streak-ingest --test sensitive -- --exact an_unresolved_deck_or_an_unreadable_set_reads_as_kept_away
A4: cargo test -p deck-streak-agent --test deck_gate -- --exact a_kept_away_deck_is_withheld_before_the_runner_starts
A5: cargo test -p deck-streak-agent --test deck_gate -- --exact an_unreadable_gate_is_withheld_before_the_runner_starts
A6: cargo test -p deck-streak-agent --test deck_gate -- --exact cards_with_no_deck_scope_are_withheld_before_the_runner_starts
A7: cargo test -p deck-streak-readings --test sensitive_day_set -- --exact a_kept_away_decks_cards_are_never_selected_into_a_day_set
A8: cargo test -p deck-streak-coordination --test readings_resolve -- --exact an_unreadable_set_resolves_no_day_and_records_nothing
A9: cargo test -p deck-streak-agent --test deck_gate_census -- --exact every_ai_call_site_passes_the_deck_gate_or_carries_no_deck_content
A10: cargo test -p deck-streak-api --test sensitive_decks -- --exact a_cross_site_or_signed_out_change_marks_nothing
A11: cargo test -p deck-streak-ingest --test sensitive -- --exact the_marks_are_exported_and_an_erase_leaves_every_deck_readable
A12: pnpm exec vitest run web/app/src/lib/study/ai-decks.test.ts -t "a deck's switch is off until it is turned on, and turning it on saves the mark"
A13: pnpm exec vitest run web/app/src/lib/study/ai-decks.test.ts -t "a deck under a kept-away deck shows on and cannot be changed"
A14: pnpm exec vitest run web/app/src/lib/study/ai-decks.test.ts -t "a failed save turns the switch back and says so"
A15: pnpm exec vitest run web/app/src/lib/study/ai-decks-locales.test.ts -t "every locale holds every AI-and-your-decks key"
```

Each criterion is red first against a stub that keeps every input and lacks only the behaviour, so
each red is an assertion on that behaviour, never a build failure. A9 is committed alone, before the
stubs. `docs/red-first/SPEC-381.md` quotes each red and each green.

## 4. File manifest

| file | context | change |
|---|---|---|
| `migrations/038101_ingest_sensitive_decks.sql` | `deck-streak-ingest` | added: the `sensitive_decks` table |
| `crates/ingest/src/sensitive.rs` | `deck-streak-ingest` | added: the store, `read_marked`, `admits` |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module line |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | changed: the table's export and erase |
| `crates/ingest/tests/sensitive.rs` | `deck-streak-ingest` | added: A2, A3, A11 |
| `crates/agent/src/deck_gate.rs` | `deck-streak-agent` | added: `DeckScope`, `DeckGate`, `DeckVerdict` |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the module line and its exports |
| `crates/agent/src/duty.rs` | `deck-streak-agent` | changed: the engine's gate field, `DutyInput`'s scope, the gate in `decide` |
| `crates/agent/tests/deck_gate.rs` | `deck-streak-agent` | added: A4, A5, A6 |
| `crates/agent/tests/deck_gate_census.rs` | `deck-streak-agent` | added: A9 |
| `crates/agent/tests/fixtures/census/a-second-launch.rs.fixture` | `deck-streak-agent` | added: a planted site |
| `crates/agent/tests/fixtures/census/a-runner-before-the-gate.rs.fixture` | `deck-streak-agent` | added: a planted site |
| `crates/agent/tests/fixtures/census/an-undeclared-runner.rs.fixture` | `deck-streak-agent` | added: a planted site |
| `crates/agent/tests/fixtures/census/an-undeclared-memory.rs.fixture` | `deck-streak-agent` | added: a planted site |
| `crates/agent/tests/fixtures/census/an-undeclared-tool.rs.fixture` | `deck-streak-agent` | added: a planted site |
| `crates/agent/tests/support/mod.rs` | `deck-streak-agent` | changed: an admitting gate for the shipped tests |
| `crates/agent/tests/duty.rs` | `deck-streak-agent` | changed: the engine is built with its gate |
| `crates/agent/tests/redteam.rs` | `deck-streak-agent` | changed: the engine is built with its gate |
| `crates/readings/src/day_set.rs` | `deck-streak-readings` | changed: `hold_back_sensitive` |
| `crates/readings/tests/sensitive_day_set.rs` | `deck-streak-readings` | added: A7 |
| `crates/coordination/src/readings/resolve.rs` | `deck-streak-coordination` | changed: the set is read first |
| `crates/coordination/src/sensitive_decks.rs` | `deck-streak-coordination` | added: list, mark, unmark |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module line |
| `crates/coordination/tests/readings_resolve.rs` | `deck-streak-coordination` | changed: A8 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: the new table's disposition |
| `crates/api/src/sensitive_decks_routes.rs` | `deck-streak-api` | added: the two routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes are merged |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module line |
| `crates/api/tests/sensitive_decks.rs` | `deck-streak-api` | added: A1, A10 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the `DeckGate` impl |
| `crates/kernel/tests/schema.rs` | `deck-streak-kernel` | changed: the table's register row |
| `.sqlx/` | workspace | added: one offline entry per new checked query |
| `web/app/src/routes/study/ai-decks/+page.svelte` | `miniapp` | added: the screen |
| `web/app/src/lib/study/AiDecks.svelte` | `miniapp` | added: the deck tree with its switches |
| `web/app/src/lib/study/ai-decks.ts` | `miniapp` | added: the marked set, inheritance, save and revert |
| `web/app/src/lib/study/ai-decks.test.ts` | `miniapp` | added: A12, A13, A14 |
| `web/app/src/lib/study/ai-decks-locales.test.ts` | `miniapp` | added: A15 |
| `web/app/src/lib/study/DeckList.svelte` | `miniapp` | changed: the link |
| `web/app/src/lib/api.ts` | `miniapp` | changed: `sensitiveDecks` and `setSensitive` |
| `web/app/src/lib/api.test.ts` | `miniapp` | changed: the two calls' paths, methods and bodies |
| `web/app/src/lib/routes.ts` | `miniapp` | changed: `/study/ai-decks` |
| `web/app/src/lib/startapp.test.ts` | `miniapp` | changed: `BY_PATH` lists `/study/ai-decks`, one entry and one comment line; no assertion removed |
| `web/app/messages/en.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/zh-Hans.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/zh-Hant.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/ja.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/ko.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/fr.json` | `miniapp` | changed: the screen's keys |
| `web/app/messages/es.json` | `miniapp` | changed: the screen's keys |
| `privacy.json` | docs | changed: the `sensitive-decks` category |
| `PRIVACY.md` | docs | changed: its row |
| `docs/CONTEXT-MAP.md` | docs | changed: the table's register row |
| `docs/schematics/web-study-screens.md` | docs | changed: the new last section, insert-only |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | changed: the web client's new rows |
| `formal/tla/SensitiveDeckGate/SensitiveDeckGate.tla` | formal | added |
| `formal/tla/SensitiveDeckGate/MCSensitiveDeckGate.cfg` | formal | added |
| `formal/tla/SensitiveDeckGate/witness/a-gate-that-trusts-the-selection.cfg` | formal | added |
| `formal/tla/SensitiveDeckGate/witness/an-unreadable-set-admits.cfg` | formal | added |
| `formal/tla/SensitiveDeckGate/witness/cards-without-a-scope-are-admitted.cfg` | formal | added |
| `formal/lean/Formal/SensitiveDeck.lean` | formal | added |
| `formal/lean/Formal/SensitiveDeckVectors.lean` | formal | added |
| `formal/vectors/sensitive-deck.jsonl` | formal | added |
| `formal/lean/Formal.lean` | formal | changed: the import line |
| `docs/specs/SPEC-381-a-deck-the-learner-keeps-away-from-ai-reaches-no-ai-duty-and-every-deck-starts-readable.md` | docs | added |
| `docs/decisions/ADR-392-the-mark-lives-on-the-server-and-one-deck-gate-stands-before-every-ai-runner.md` | docs | added |
| `docs/red-first/SPEC-381.md` | docs | added |
| `scripts/mutation-rows.d/S38100-S38199.json` | scripts | added |
| `changelog.d/sensitive-deck-381.md` | docs | added |

## 5. What this does NOT cover

- No setting on the native clients: they hold no client of DeckStreak's API, and native sign-in to
  it is its own work (#58). The gate holds their decks all the same, because it reads the server's
  marks.
- No new AI duty. The card generation (#32), the regeneration (#34) and the comeback (#35) duties
  each join the census when they are built, and pass the gate by the rule R6 pins.
- No vault duty. The inbox curator (#50), the daily note and weekly synthesis (#49) and the vault
  pass (#54) read notes, not decks, and each joins the census when it is built.
- No stats bridge (#153) and no flashcard candidates (#65): each joins the census when it is built.
- No AI persona settings in the clients (#162, #43), no reading in the clients (#45) and no persona
  creation (#51).
- No memory port beyond the one on `dev`: the leech (#133), lapse (#83) and live band (#85) ports
  carry deck content and must each be declared `deck-gate` in the census when they land.
- No census read of drill-named sources: the census skips them by name while the drill surface is
  on hold and prints how many it skipped; the fail-closed gate in `decide` is the control for an AI
  call the census cannot see (#46).

## 6. Risks

- **A caller declares a scope narrower than its cards.** The census proves the gate is asked, not
  that a future caller's scope is whole. Each later duty builds its scope from the same records its
  cards come from (the day set carries both ids per card), and its own SPEC owes the test.
- **A mark made while a run is in flight.** A run whose gate read the set before the mark committed
  may send that deck's cards once; every run whose gate reads after it does not. The TLA+ entry
  states this window, and section 8's first witness shows why the gate re-reads rather than trusting
  the selection.
- **The census misses a new shape.** A model reached by a shape the census does not list is a gap.
  The shapes are the launch script's name, the runner trait, process launches, memory ports, MCP
  tools and the engine literal; a new kind of call joins the list in its own SPEC.
- **Cited lines move.** The threat model cites `path:line`; a moved line reddens its test in CI.
  The build re-measures each cited line in every file it edits.

## 7. What only CI proves

The web suite's browser run, the web mutation job and the threat model's citation test are CI's,
read by name.

## 8. Formal model

- **TLA+, `formal/tla/SensitiveDeckGate`.** The learner's mark and unmark, a day set's selection
  and a duty's gate are actors over one shared set, and the gate is a check followed by an act. The
  entry models two runs and two decks (one under the other), a read that can fail, and a run whose
  cards may carry no scope. Its properties, at `ramp=report`: `NoMarkedDeckIsSent` (a deck marked
  before a run's gate read is never in what that run sends), `AnUnreadableSetSendsNothing` and
  `NoUnscopedCardIsSent`. Its witnesses: `a-gate-that-trusts-the-selection` (the gate reads the
  set only at selection), `an-unreadable-set-admits` and `cards-without-a-scope-are-admitted`. It
  covers `crates/agent/src/duty.rs` (`decide`), `crates/ingest/src/sensitive.rs` (`read_marked`,
  `admits`), `crates/readings/src/day_set.rs` (`hold_back_sensitive`) and
  `crates/daemon/src/wiring.rs` (`judge_deck_scope`), and cites #751.
- **Lean, `formal/lean/Formal/SensitiveDeck.lean`.** `admits` is a total function, ported with its
  vectors. Its theorems, at `ramp=report`: a card is refused when an ancestor or the deck itself is
  marked, for its home or its current deck; marking more decks never admits a card the smaller set
  refused; an unresolved deck is refused. It covers `crates/ingest/src/sensitive.rs` (`admits`).
- No existing cover names a file this delivery edits: `git grep -n '@phx covers' e7ecf10d --
  formal` reads none for `crates/agent/`, `crates/readings/`, `crates/coordination/src/readings/`,
  `crates/api/src/router.rs`, `crates/daemon/src/wiring.rs` or `crates/ingest/src/data_rights.rs`.

## 9. Mutation rows

| stem | file | mutant | killer |
|---|---|---|---|
| `S38100-A-KEPT-AWAY-DECK-IS-SENT` | agent `src/duty.rs` | the `deck-sensitive` arm goes on to the input gate | `deck_gate::a_kept_away_deck_is_withheld_before_the_runner_starts` |
| `S38101-AN-UNREADABLE-GATE-ADMITS` | agent `src/duty.rs` | the `deck-unreadable` arm goes on to the input gate | `deck_gate::an_unreadable_gate_is_withheld_before_the_runner_starts` |
| `S38102-AN-UNSCOPED-CARD-IS-ADMITTED` | agent `src/duty.rs` | the empty-scope test reads false | `deck_gate::cards_with_no_deck_scope_are_withheld_before_the_runner_starts` |
| `S38103-A-CHILD-ESCAPES-ITS-PARENT` | ingest `src/sensitive.rs` | the ancestor walk reads the deck itself only | `sensitive::a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed` |
| `S38104-THE-HOME-DECK-IS-NOT-READ` | ingest `src/sensitive.rs` | the rule reads the current deck only | `sensitive::a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed` |
| `S38105-THE-CURRENT-DECK-IS-NOT-READ` | ingest `src/sensitive.rs` | the rule reads the home deck only | `sensitive::a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed` |
| `S38106-AN-UNRESOLVED-DECK-IS-ADMITTED` | ingest `src/sensitive.rs` | an unresolved deck reads as admitted | `sensitive::an_unresolved_deck_or_an_unreadable_set_reads_as_kept_away` |
| `S38107-ERASE-KEEPS-THE-MARKS` | ingest `src/data_rights.rs` | the erase skips the table | `sensitive::the_marks_are_exported_and_an_erase_leaves_every_deck_readable` |
| `S38108-THE-DAY-SET-KEEPS-A-KEPT-AWAY-CARD` | readings `src/day_set.rs` | `hold_back_sensitive` keeps every card | `sensitive_day_set::a_kept_away_decks_cards_are_never_selected_into_a_day_set` |
| `S38109-AN-UNREADABLE-SET-RESOLVES-A-DAY` | coordination `src/readings/resolve.rs` | a failed read becomes an empty set | `readings_resolve::an_unreadable_set_resolves_no_day_and_records_nothing` |
| `S38110-A-CROSS-SITE-MARK-IS-ACCEPTED` | api `src/sensitive_decks_routes.rs` | the PUT handler drops the state-change check | `sensitive_decks::a_cross_site_or_signed_out_change_marks_nothing` |

Each row's find occurs exactly once in its file, and each is killed by hand before it is committed.
A key held inside a compile-time-checked query cannot be planted; such a row re-spells its statement
as a runtime query. The web files are judged by the web mutation run at a break of 100, with no
hand row.
