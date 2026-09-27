# SPEC-045: each study day, every topic's new cards come from the scheduler's own queue and end in one honest state

- **Wave:** W1. **Issue:** #31 (epic #2). **Context(s):** `deck-streak-readings` (the rules and their tables); `deck-streak-coordination` (the resolve use case).
- **Decided by:** ADR-009 (ingest syncs a private copy and asks Anki's own scheduler for the new-card
  queue), ADR-012 (the parity oracle), ADR-019 (readings generate whenever the last sync succeeded,
  and pause after two days without study), and ADR-045 (the order of the gates, the pause's source,
  the private taxonomy and the closed states).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-045.md` (ADR-016).

## 1. The problem, measured

- **No reading has ever reached the owner.** The predecessor's lane fired every night and refused
  every topic on a freshness gate that judged the collection file's age, while its own sync ledger
  showed a healthy rail (ADR-019; the second-brain inventory, private). The owner decided that a
  reading generates whenever the last sync succeeded, and that after two or more days without study
  the daily readings pause.
- **What is ported.** The day set is the scheduler's own queue of today's new cards, one query per
  top-level deck because children share their parent's new-card budget, each card attributed by its
  original deck (`pipeline_layers/preread.py:PreReadLayer._resolve_day_set`,
  `prereading.py:resolve_day_sets`); the topic of a law deck (`leeches.py:_law_subject`) and of a
  language or writing deck (`prereading.py:_topic_key_for_deck`); the digest of a day set
  (`prereading.py:_digest_for_card_ids`); the 30-second resolve budget and the 1000-card fetch cap
  per root; and the honest distinction between nothing to read, could not tell and failed.
- **A waste not to port.** The predecessor copied the whole collection before it refused. Here the
  sync and pause gates run before any collection work.
- **What the parity oracle proves.** `prereading.py:resolve_day_sets`, `leeches.py:_law_subject` and
  `prereading.py:_digest_for_card_ids`, each generated through an adapter that replaces the
  predecessor's private deck constants with a synthetic taxonomy, recorded in the golden's note, so
  no golden carries a real deck name.
- **Prerequisites.** SPEC-020 (the study day, the clock, the offload rail, configuration), SPEC-022
  (the sync and its ledger, which answers whether the last sync succeeded), SPEC-023 (the read-only
  reviews and deck names, and the new-card queue per root), SPEC-021 (export and erase) and SPEC-029
  (the golden reader). It is the first readings SPEC; SPEC-046 to SPEC-053 build on it.

## 2. Requirements

R1. Topics come from a private taxonomy file (schema `deckstreak.readings.taxonomy.v1`, its path
    given by configuration): the law deck roots and band names, each language deck with its code and
    the note field that holds its term, and the writing roots that fold into a language. The
    synthetic `deploy/config/readings-taxonomy.example.json` shows its shape. No deck name and no
    topic list is a literal in code or anywhere in the repository.
R2. Under a configured law root, a deck's topic is `law/<slug>`, where the subject is the fourth
    path segment when the second is a configured band (the last segment when the path is shorter),
    and the second segment otherwise; the bare root maps to no topic. A language deck maps to
    `language/<code>`, and a writing deck whose second segment is a language's display name folds
    into that language. A slug is the subject lowercased with each run of spaces, underscores and
    hyphens collapsed to one hyphen, and it must match `[a-z0-9]+(-[a-z0-9]+)*`; a deck whose slug
    does not is reported as unmapped and never forced into a key. The mapping equals the golden of
    `leeches.py:_law_subject` through the adapter.
R3. The day set is resolved with one query per top-level deck other than `Default`, through
    ingest's new-card queue, at most 1000 cards per root. A root whose answer holds fewer new cards
    than the scheduler's own new count, or 1000 or more when that count is absent, is saturated.
    Each card is attributed by its original deck (the original deck id when set, else its deck); a
    card an earlier root already claimed is skipped; a topic's digest is the SHA-256 of its sorted
    card ids joined by commas. The resolution equals the golden of `prereading.py:resolve_day_sets`
    through the adapter.
R4. The gates run before any collection work, in this order: (1) when the last sync did not succeed,
    every topic is `could_not_tell` with class `rail_broken` and reason `sync_failed`; (2) when the
    owner made no qualifying review (a review of type 0 to 3 with ease 1 or more) on each of the two
    study days before this one, every topic is `paused`; (3) otherwise the day set is resolved. The
    collection file's age is never a reason to refuse.
R5. Each topic of the taxonomy ends a study day in exactly one state: `ready`, `no_new_cards`,
    `could_not_tell` with a class and a closed reason, `paused`, or `failed` with a closed reason
    (`failed` and `ready` are set by SPEC-046). The state is one enum whose variants carry their
    class and reason, so a topic cannot hold two states and an unknown reason cannot be written.
R6. The `could_not_tell` reasons and their classes are closed: `sync_failed`, `collection_locked`,
    `collection_open_failed` and `day_set_resolve_timeout` are `rail_broken`; `taxonomy_missing` and
    `day_set_fetch_saturated` are `config_fault`.
R7. The resolution runs on the kernel's offload rail under a 30-second budget (the predecessor's
    `preread.py:PREREAD_OFFLOAD_BUDGET_S`); past it, every unresolved root is `day_set_resolve_timeout`.
R8. Readings owns `reading_topic_days` (one row per study day and topic: state, class, reason,
    digest, card ids, note ids, new-card count, run id, `created_at`) and `reading_runs` (one row per
    run: trigger, study day, start, end, outcome, `created_at`), both created `STRICT` by
    `migrations/004501_readings_topic_days_and_runs.sql`. Both are registered in the context map's
    ownership register (the predecessor's `preread_runs` maps onto them), declared in
    `privacy.json`, and exported and erased by readings' data-rights port.
R9. Decks with new cards that map to no topic are counted with the run, their names kept to the
    private log, and are never alerted as a failure.
R10. Pause is the readings' own rule, computed from ingest's reviews. It is not the governor's lapse:
    it never mints, reads or stores a lapse id (docs/CONTEXT-MAP.md, "Overloaded words").
R11. The rules live in the readings context, which depends on the kernel and ingest only; the
    use case that runs a resolution for a study day lives in coordination.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | on a synthetic collection, the day set is the scheduler's queue per root under a shared parent budget, attributed by each card's original deck | `the_day_set_is_the_schedulers_queue_per_root_attributed_by_original_deck` |
| A2 | the day-set resolution equals the golden of `prereading.py:resolve_day_sets` | `the_day_set_resolution_matches_the_parity_golden` |
| A3 | topic keys equal the golden of `leeches.py:_law_subject`, and an unsafe slug is reported unmapped | `topic_keys_match_the_parity_golden` |
| A4 | the digest equals the golden of `prereading.py:_digest_for_card_ids` | `the_digest_matches_the_parity_golden` |
| A5 | with the last sync successful, the day set is resolved however old the collection file is (injected clock) | `a_healthy_sync_resolves_whatever_the_collection_files_age` |
| A6 | with the last sync failed, every topic is `could_not_tell` `rail_broken` `sync_failed` and the queue port is never called | `a_failed_last_sync_is_could_not_tell_before_any_collection_work` |
| A7 | with no qualifying review on the two study days before, every topic is `paused`; one review on either day resumes (injected clock) | `two_days_without_study_pause_every_topic` |
| A8 | every state, class and reason is stored and read back distinctly, and a planted mapping that collapses two fails (examined count reported) | `every_state_and_class_is_stored_and_read_back_distinctly` |
| A9 | a saturated root is `could_not_tell` `config_fault` `day_set_fetch_saturated` | `a_saturated_root_is_a_config_fault` |
| A10 | a resolution past its 30-second budget is `rail_broken` `day_set_resolve_timeout` (paused test time) | `a_resolution_past_its_budget_is_rail_broken` |
| A11 | topics come only from the configured taxonomy: two synthetic taxonomies over one collection give two topic sets, and no taxonomy gives `config_fault` `taxonomy_missing` | `the_topics_come_only_from_the_configured_taxonomy` |
| A12 | the public scrub is green over the tree, the example taxonomy included | the public scrub; `test_the_public_scrub_is_green_with_the_example_taxonomy` |
| A13 | readings' data-rights port exports and erases `reading_topic_days` and `reading_runs` | `the_topic_days_and_runs_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-readings --test day_set -- --exact the_day_set_is_the_schedulers_queue_per_root_attributed_by_original_deck
A2: cargo test -p deck-streak-readings --test day_set -- --exact the_day_set_resolution_matches_the_parity_golden
A3: cargo test -p deck-streak-readings --test topics -- --exact topic_keys_match_the_parity_golden
A4: cargo test -p deck-streak-readings --test day_set -- --exact the_digest_matches_the_parity_golden
A5: cargo test -p deck-streak-readings --test gates -- --exact a_healthy_sync_resolves_whatever_the_collection_files_age
A6: cargo test -p deck-streak-readings --test gates -- --exact a_failed_last_sync_is_could_not_tell_before_any_collection_work
A7: cargo test -p deck-streak-readings --test gates -- --exact two_days_without_study_pause_every_topic
A8: cargo test -p deck-streak-readings --test states -- --exact every_state_and_class_is_stored_and_read_back_distinctly
A9: cargo test -p deck-streak-readings --test day_set -- --exact a_saturated_root_is_a_config_fault
A10: cargo test -p deck-streak-readings --test day_set -- --exact a_resolution_past_its_budget_is_rail_broken
A11: cargo test -p deck-streak-readings --test topics -- --exact the_topics_come_only_from_the_configured_taxonomy
A12: python3 -m unittest discover -s scripts/tests -p test_readings_taxonomy_scrub.py -k test_the_public_scrub_is_green_with_the_example_taxonomy
A13: cargo test -p deck-streak-readings --test rights -- --exact the_topic_days_and_runs_are_exported_and_erased
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/Cargo.toml` | `deck-streak-readings` | changed: the workspace dependencies it uses |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed: the modules below |
| `crates/readings/src/taxonomy.rs` | `deck-streak-readings` | added: the private taxonomy file |
| `crates/readings/src/topic.rs` | `deck-streak-readings` | added: deck to topic |
| `crates/readings/src/day_set.rs` | `deck-streak-readings` | added: roots, queue queries, attribution, digest, saturation |
| `crates/readings/src/gates.rs` | `deck-streak-readings` | added: the last-sync and pause gates |
| `crates/readings/src/state.rs` | `deck-streak-readings` | added: the closed topic states |
| `crates/readings/src/store.rs` | `deck-streak-readings` | added: `reading_topic_days` and `reading_runs` |
| `crates/readings/src/rights.rs` | `deck-streak-readings` | added: the data-rights port |
| `migrations/004501_readings_topic_days_and_runs.sql` | `deck-streak-readings` | added |
| `crates/readings/tests/day_set.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/topics.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/gates.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/states.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/rights.rs` | `deck-streak-readings` | added |
| `crates/coordination/src/readings/mod.rs` | `deck-streak-coordination` | added |
| `crates/coordination/src/readings/resolve.rs` | `deck-streak-coordination` | added: resolve a study day's topics and record them |
| `deploy/config/readings-taxonomy.example.json` | deploy | added: a synthetic taxonomy |
| `scripts/tests/test_readings_taxonomy_scrub.py` | repo | added |
| `tools/parity-oracle/registry/spec_045.py` | repo | added: registers the three functions with the synthetic-taxonomy adapter (SPEC-029's registry) |
| `tools/parity-oracle/goldens/resolve_day_sets.json` | repo | added |
| `tools/parity-oracle/goldens/_law_subject.json` | repo | added |
| `tools/parity-oracle/goldens/_digest_for_card_ids.json` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register gains `reading_topic_days` and `reading_runs` |
| `privacy.json` | repo | changed: the readings day-set categories |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/schematics/reading-lifecycle.md` | docs | added: the reading's and the topic day's state machines |
| `docs/specs/SPEC-045-readings-day-set-and-honest-states.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-045-day-set-gates-pause-source-and-taxonomy.md` | docs | added |
| `docs/red-first/SPEC-045.md` | docs | added |

## 5. What this does NOT do

- It generates no reading and writes no vault note (#32).
- It treats no skip day specially: a skip day counts as neither study nor silence once the skip day
  exists (#108).
- It decides no lapse and offers no comeback reading (#83, #35).
- It pages nobody for a could-not-tell class; the health check does (#36).
- It shows no state in the Mini App or the bot (#37, #38).
- It schedules nothing; the jobs do (#39).

## 6. Risks

- **Anki's engine answers the queue differently from the predecessor's library.** Detected by A1 on
  a synthetic collection built through ingest, beside the pure golden A2.
- **An adapter hides a private constant in a golden.** Prevented by the generator's recorded adapter
  note and synthetic names; detected by the public scrub over the goldens (A12).
- **The private taxonomy is absent on the host.** Every topic is `config_fault` `taxonomy_missing`,
  and the health check pages it (SPEC-050).
- **Reviews arrive late** (the sync that carries yesterday's reviews fails), so a pause is judged on
  missing data. The last-sync gate runs first, so a failed sync is `could_not_tell`, never `paused`;
  and the owner's tap regenerates on demand whatever the gates say (SPEC-048).
