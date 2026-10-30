# SPEC-514: Searches are DeckStreak's own: one typed query, a closed operator grammar, and an evaluator the goldens pin

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-532, under ADR-029, ADR-515 and ADR-516 |
| measured at | the delivery's base `bae990a2` (every file this SPEC cites is byte-equal there), and the pinned engine `65e053b8` |
| schematic | `docs/schematics/search-from-text-to-selected-cards.md` (new, data flow); `docs/schematics/context-map.md` (changed: the search crate's node and its two edges) |
| mutation band | S51400-S51499 (no hand row: section 9) |

## 1. The problem, measured

A search is text the engine reads, and DeckStreak has no reading of its own. What was read at `bae990a2`:

- **The skip search is wrapped as text.** `skip_search` (`crates/ingest/src/skip.rs:103-108`) checks the configured
  text with `is_one_expression` (`:62-88`), a balance count of parentheses and quotes, and then formats
  `({configured}) -is:new -is:learn` followed by the holds (`:30`). The check knows no key: `dekc:Spanish`, a
  misspelled deck search, passes it, and the engine reads it as a search of a field named `dekc` that no note type
  has, so the skip selects nothing and says nothing.
- **The setting is checked by the same count.** `SkipSearch::parse` (`crates/ingest/src/settings.rs:321-327`) admits
  whatever `is_one_expression` admits; the start refusal names the setting and never the value
  (`crates/ingest/tests/settings.rs:174-200`).
- **The engine's search has goldens and no reader.** `tools/parity-oracle/goldens/anki.search.operator.json`
  (G-L6-1, SPEC-498) holds 88 cases, eight per class over eight classes (`rollover`, `filtered-deck`, `buried`,
  `suspended`, `intraday-learning`, `day-learning`, `deck-children`, `unicode-deck-names`) and 24 seeded random
  cases, each a synthetic collection, a search and the corpus indices of the cards the engine selects, in its order.
  `crates/ingest/tests/engine_goldens.rs:156` checks its answer keys; no port reads its answers.
- **The golden's clock.** The driver's harness holds each case's clock at the case's own recorded instant, its `now`,
  and the engine reads that clock and the host's zone, so each case ran at its own instant, read in the host's zone
  (UTC); no one time of day is assumed for the run. Measured: moving every case's instant by each whole minute of a
  day around its own, 643 of the 1,440 shifts (from -17,940 to 20,580 s, one band that holds the unshifted instant)
  give every case its recorded answer.
- **The engine's day has goldens too.** `anki.sched.timing_today.json` (40 cases) and `anki.timing.day.json` (48)
  record the day number and next rollover for a creation time, rollover hour, offset and instant.
- **What the engine's search compiles to** (the pinned engine's `rslib/src/search/sqlwriter.rs`): unqualified text is
  `LIKE '%text%'` over the sort field and the joined fields, ASCII letters folded; deck, note type, template, field
  and tag names are `(?i)` globs; `is:` reads the queue and type numbers; `prop:due` compares against the days
  elapsed or, in the learning queues, the truncated days from the next rollover; `added:` and `rated:` compare ids
  with the rollover's instant.
- **Not measured:** the engine's order for search shapes the goldens do not hold (section 6).

## 2. Requirements

R1. **One typed query.** `crates/search`, package `deck-streak-search`, holds `Query` (`Term`, `Not`, `And`, `Or`):
`and` binds before `or`, as the engine's SQL does, and a group keeps its own node.

R2. **A closed grammar.** The grammar reads the engine's syntax (white space joins with `and`, `or` and `and` in any
case, `-` negates, `(` `)` group, quotes, `key:"value"`, the escapes `\"`, `\:`, `\(`, `\)`, `\-`, `\\`, `\*`, `\_`)
over a closed vocabulary: unqualified text, field searches, `deck`, `tag`, `is` (`new`, `learn`, `review`, `due`,
`buried`, `suspended`), `prop` (`due`, `ivl`, `reps`, `lapses`), `card`, `note`, `added` and `rated`. Every other
engine key is refused by its own name from a fixed list; a value outside a key's list is refused naming the key.

R3. **Profiles.** A `Profile` is a constant list of operators. `Profile::SKIP` admits text, `deck`, `tag`, `is`,
`prop`, `card` and `note`; under it any other key is unknown. `Profile::OPERATOR` admits every operator, field
searches included.

R4. **Limits.** Every parse is held to `Limits`: 1024 bytes, 8 nested groups and 32 terms in one group. A search past
one is refused, never truncated.

R5. **The evaluator.** `eval(query, collection, day)` answers the indices of the cards a query selects over a
`Collection`'s rows (decks, note types, notes, cards, review log), in the engine's order: a query whose top level
holds a deck name lists the matching decks' cards by deck, queue, due and id, then the cards a filtered deck borrowed
from them; every other query lists cards by id. It reads no clock, file, environment, network, process or thread.

R6. **The engine's day.** `EngineDay::at` computes the days elapsed and the next rollover as the pinned engine does
for a collection whose creation offset is set, with the study day crate's rule (`deck-streak-study-day`); the
search crate depends on that crate alone and has no edge to the kernel.

R7. **Rust-side name matching.** Text and field values match by the engine's `LIKE` (ASCII letters folded, `%` and
`_`, `\` the escape); deck, note type, template, field and tag names by its globs with Unicode case folding. No match
runs in SQL.

R8. **The structural skip wrap.** `SkipQuery::new` parses the configured search under `Profile::SKIP`; its query is
one `and` of the configured query and the holds (`-is:new`, `-is:learn`, `prop:due=0`, `-is:suspended`,
`-is:buried`, `-deck:filtered`), and its text, byte for byte the predecessor's wrap followed by the holds, parses
back to that query. Ingest's `skip_search` wraps through it, mapping a structural refusal to `NotOneExpression` and
any other to the new `OutsideTheSkipProfile`; ingest's `SkipSearch` setting admits what `Profile::SKIP` admits.

R9. **The read-only parity tool.** `deck_streak_ingest::search_parity::search_parity` opens the source collection
only through `Db::open_foreign_read_only`, runs the engine's day and search only on a scratch copy, refuses a scratch
path that is the source, and answers three counts: match, engine-only and DeckStreak-only.

## 3. Acceptance criteria of SPEC-514

The row's eight contract items map onto six criteria: the start refusal and the limits are one criterion (A2), as are
the golden and its rollover class (A3).

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R3: the skip profile accepts and refuses each `skip_search.json` input as `is_one_expression` does | the stub's parse refuses every search | ADR-532 D3 |
| A2 | R2, R3, R4, R8: an unknown key stops start naming the setting, never the value; every key outside the grammar and every shape it cannot read is refused by name; the depth, length and arity limits refuse | the stub's parse refuses every search as blank, so a known key stops start too and every refusal is the wrong one | ADR-532 D2, D3 |
| A3 | R5, R6, R7: `eval` equals G-L6-1, all 88 cases; its rollover class passes; the engine's day equals both timing goldens | the stub's parse refuses every golden search, and its day is zero | ADR-532 D1, D5, D6 |
| A4 | R8: the holds cannot be escaped: a property over seeded random configured predicates, and planted `) or (` texts refused before the wrap while their text wrap escapes | the stub wraps nothing | ADR-532 D4 |
| A5 | K-operator-profile passes | VOID: no Kani run exists for this tree yet (section 8) | ADR-532 D7 |
| A6 | R9: the parity tool opens read-only, refuses a writable open, and prints only counts | the stub's tool refuses every read | ADR-532 D8 |

```acceptance
A1: cargo test -p deck-streak-ingest --test search_profile -- --exact the_skip_profile_accepts_and_refuses_each_golden_skip_search_as_the_one_expression_check_does
A2: cargo test -p deck-streak-ingest --test search_profile -- --exact an_unknown_key_stops_start_naming_the_setting_never_the_value
A2: cargo test -p deck-streak-ingest --test search_profile -- --exact a_search_the_skip_profile_refuses_is_refused_before_the_wrap
A2: cargo test -p deck-streak-search --test grammar -- --exact a_key_outside_the_grammar_is_refused_by_name
A2: cargo test -p deck-streak-search --test grammar -- --exact depth_length_and_arity_limits_refuse
A2: cargo test -p deck-streak-search --test grammar -- --exact every_shape_refusal_is_named_and_carries_no_text
A3: cargo test -p deck-streak-search --test eval_golden -- --exact eval_equals_the_engines_search_golden
A3: cargo test -p deck-streak-search --test eval_golden -- --exact the_run_second_is_one_of_the_seconds_the_golden_admits
A3: cargo test -p deck-streak-search --test eval_golden -- --exact the_rollover_class_passes
A3: cargo test -p deck-streak-search --test eval_golden -- --exact the_engine_day_matches_the_engines_timing_golden
A3: cargo test -p deck-streak-search --test grammar -- --exact the_grammar_reads_the_engines_syntax_into_one_typed_query
A3: cargo test -p deck-streak-search --test profile -- --exact each_profile_admits_exactly_its_listed_operators
A4: cargo test -p deck-streak-search --test holds -- --exact the_holds_cannot_be_escaped_by_any_configured_predicate
A4: cargo test -p deck-streak-search --test holds -- --exact a_planted_closing_parenthesis_is_refused_and_would_escape_a_text_wrap
A6: cargo test -p deck-streak-ingest --test search_parity -- --exact the_parity_tool_opens_read_only_refuses_a_writable_open_and_prints_only_counts
```

A5 has no command: its harness is VOID until a Kani run is pinned for this tree (section 8). The existing suites that
read the changed code (`--test skip_record`, `--test settings` of `deck-streak-ingest`; `--test skip_flow` of
`deck-streak-coordination`) run unchanged and stay green.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/search/Cargo.toml` | search | new: the study day crate its one dependency; the golden reader's two dev-dependencies |
| `crates/search/src/lib.rs` | search | new: the crate's doc and its modules |
| `crates/search/src/query.rs` | search | new: `Query`, `Term`, `State`, `Prop`, `Compare`, `Template`, `Operator` |
| `crates/search/src/parse.rs` | search | new: the grammar, `Limits` and `Refusal` |
| `crates/search/src/profile.rs` | search | new: `Profile::SKIP` and `Profile::OPERATOR` |
| `crates/search/src/collection.rs` | search | new: the rows a search is evaluated over |
| `crates/search/src/day.rs` | search | new: `EngineDay` |
| `crates/search/src/pattern.rs` | search | new: `like`, `to_like`, `Glob`, `strip_html` |
| `crates/search/src/eval.rs` | search | new: `eval`, `selects` and the engine's order |
| `crates/search/src/skip.rs` | search | new: `SkipQuery`, `holds`, `HOLDS`, `REVIEW_ONLY` |
| `crates/search/tests/grammar.rs` | search tests | new: A2's refusals and limits, A3's parse |
| `crates/search/tests/profile.rs` | search tests | new: A3's profile census |
| `crates/search/tests/eval_golden.rs` | search tests | new: A3 |
| `crates/search/tests/holds.rs` | search tests | new: A4 |
| `crates/ingest/Cargo.toml` | ingest | changed: the edge to the search crate |
| `crates/ingest/src/skip.rs` | ingest | changed: `skip_search` parses through `SkipQuery` and wraps with `wrap_search`; `SearchRefusal::OutsideTheSkipProfile`; the holds' text is the search crate's |
| `crates/ingest/src/settings.rs` | ingest | changed: `SkipSearch::parse` admits what `Profile::SKIP` admits |
| `crates/ingest/src/search_parity.rs` | ingest | new: the read-only parity tool |
| `crates/ingest/src/engine.rs` | ingest | changed: `RslibEngine::scratch_search`, the parity tool's one call into the engine's day and due cards |
| `crates/ingest/src/lib.rs` | ingest | changed: the `search_parity` module |
| `crates/ingest/tests/search_profile.rs` | ingest tests | new: A1, A2 |
| `crates/ingest/tests/search_parity.rs` | ingest tests | new: A6 |
| `formal/lean/Formal/SearchHolds.lean` | formal | new: `holds_inescapable` and its witness |
| `formal/obligations.json` | formal | changed: O14 lists `lean/SearchHolds` present; the entry's row |
| `scripts/tests/test_formal_obligations.py` | formal census | changed: `lean/SearchHolds` among the new entries |
| `Cargo.toml` | workspace | changed: the crate's workspace dependency entry |
| `Cargo.lock` | workspace | changed: the crate's package and ingest's edge to it |
| `docs/CONTEXT-MAP.md` | docs | changed: the crate's fence row, ingest's edge and the prose |
| `docs/schematics/context-map.md` | docs | changed: the crate's node, its layer and two arrows |
| `docs/schematics/search-from-text-to-selected-cards.md` | docs | new: the data flow |
| `docs/specs/SPEC-514-searches-are-deckstreaks-own-one-typed-query-a-closed-operator-grammar-and-an-evaluator-the-goldens-pin.md` | docs | new: this file |
| `docs/decisions/ADR-532-searches-are-read-once-into-a-typed-query-under-a-closed-grammar-and-evaluated-over-rows-in-rust.md` | docs | new: the decisions |
| `docs/red-first/SPEC-514.md` | docs | new: the red-first record |
| `changelog.d/searches-typed-query-514.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- Replacing the engine's search anywhere it runs: the skip still runs the wrapped text on the engine, and the clients
  still search through it. The evaluator is held to the engine by the goldens and the parity tool, and replaces
  nothing yet.
- The engine operators outside the grammar (`flag`, `resched`, `edited`, `introduced`, `did`, `mid`, `nid`, `cid`,
  `re`, `nc`, `sc`, `w`, `dupe`, `has-cd`, `preset`, `prop:` beyond its four, `is:` beyond its six, `deck:current`,
  the `re:` and `nc:` modes): each is refused by name.
- NFC normalization of a search or a field: the golden's texts are all NFC (0 of its strings change under NFC), and
  the engine stores note text in NFC.
- The browser golden (G-L6-2, `anki.search.browser.json`) and its sort column.
- A property-testing crate: A4 runs over a seeded generator written in its test, since the lock admits no
  property-testing crate; its admission is a lock change with its own decision record.

## 6. Risks

- **The engine's order for other shapes.** The order rule is pinned by the goldens' shapes (a top-level deck name, or
  none); a deck name inside an `or`, or two deck names, may list in another order in the engine. The parity tool
  compares sets, so the counts are unaffected.
- **The golden's clock.** The driver holds each case's clock at the case's own recorded instant, so A3 does not
  depend on the run's time of day. The census test moves every case's instant by each minute of a day and holds the
  unshifted instant inside the one band that gives every case (643 of 1,440 shifts, from -17,940 to 20,580 s); a
  driver that read the clock another way would move cases at a rollover, and the census would show it.
- **A refusal that leaks the search.** Every `Refusal` variant is text-free; the grammar test checks each refusal's
  message carries none of its search, and A2 checks the start refusal names the setting only.
- **The skip text drifts from the query.** A4 parses every wrapped text back and compares it with the wrapped query.
- **The parity tool's sort field.** It reads every note type's first field as the sort field (section 1 of its
  module); a text search over a note type sorted by another field may count a note on one side only.

## 7. What only CI proves

The whole workspace's clippy and tests (`rust`), the release profile's tests (`release`), and the mutation jobs over
the new and changed files (`mutation-rust`), each read by name at the head. The engine and study web suites are not
touched by this change.

## 8. Formal model

- **Lean: `lean/SearchHolds`, present.** `holds_inescapable` proves that for every configured query and every
  answer of each term, a card the wrapped query selects meets every hold; its witness, the parse of a text wrap of
  `X) or (Y`, refutes the same claim. The entry covers `SkipQuery::query` and `holds` in `crates/search/src/skip.rs`.
  It is listed present under O14, whose status stays `planned`: `lean/SearchCompile` and the two Kani harnesses are
  still planned.
- **Kani: K-operator-profile, VOID.** No Kani run is pinned for this tree yet; the harness is unratcheted and
  nothing runs. Its owner is the delivery that pins Kani in the formal checker.
- **TLA+: NOT APPLICABLE.** A parse and an evaluation are pure functions; nothing interleaves.

## 9. Mutation rows

None. The new crate's code is generated by cargo-mutants, and its own tests (A2 to A4) are their killers. The band
S51400-S51499 holds no row.
