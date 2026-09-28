# SPEC-055: the engine moves to 26.09.3 together with the predecessor, on a patched fork until upstream carries the rebuild fix

- **Wave:** W0. **Issue:** #235 (epic #1). **Context(s):** `deck-streak-ingest` (the engine pin);
  `repo` (`Cargo.toml`, `Cargo.lock`, `deny.toml`, the engine's measurement).
- **Decided by:** ADR-058 (this SPEC's: the engine pins a patched fork of 26.09.3 until upstream
  carries the fix), ADR-022 (the spike's protocol and budgets, whose pin rule ADR-058 amends),
  ADR-009 (the engine), ADR-037 (never upload), ADR-012 (the parity oracle), ADR-018 (licences) and
  ADR-055 (CI's jobs and caches).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-055.md` (ADR-016). No code until the
  maintainer accepts this plan.

## 1. The problem, measured

- **The decision.** The owner decided to move Anki's engine from tag `26.05` to `26.09.3`, and the
  predecessor with it in the same window, so that the parity goldens keep describing the engine
  DeckStreak runs. The engine's rebuild on every cargo command (#228) is fixed two ways at once: the
  maintainer submits the fix upstream, and DeckStreak carries it on a fork until an upstream release
  holds it (ADR-058).
- **The releases in range.** Anki published no 26.06 or 26.07. The range is 26.08b1, 26.08b2, 26.08,
  26.08.1, 26.09b1 to 26.09b3, 26.09, 26.09.1 (skipped for a Windows packaging issue), 26.09.2 and
  26.09.3, whose notes are on GitHub's release pages (§9). The tree is compared directly: `26.05` is
  `e64c6b1`, `26.09.3` is `29bb700`, and neither is an ancestor of the other, because Anki tags each
  release on its own branch. `git diff --stat 26.05 26.09.3 -- rslib/` touches 82 files with 7,556
  lines added and 384 removed, and most of the added lines are new service-layer tests.
- **The rebuild is still there.** Both causes #228 names are byte-identical at both tags and on
  upstream `main` (`1f7c8d7c4`): `rslib/io/src/lib.rs:353-355` registers every file the `anki_proto`
  build script writes, and `rslib/proto_gen/src/lib.rs:253` writes the prost output a second time,
  so its mtime always postdates the run. The second watch, `rslib/build.rs:13`, is inert for a git
  dependency, because cargo skips mtime checks for paths under `$CARGO_HOME` (rust-lang/cargo#11613).
  A pin bump alone therefore saves nothing: measured on the maintainer's machine, a no-op
  `cargo build -p deck-streak-ingest` takes 31 to 35 s at 26.09.3, with the same `Dirty` reasons as
  at 26.05, and every unit is Fresh in 0.33 s once the first cause is neutralised (§7). The one-hunk
  fix in `rslib/io/src/lib.rs` applies unchanged to `26.09.3` (`git apply --check`).

What the tag diff changes, by the areas DeckStreak depends on (file:line at `26.05`, then at
`26.09.3`):

| area | what changed | what it means here |
|---|---|---|
| scheduling and FSRS | `fsrs` 5.2.0 becomes 6.6.2 (`Cargo.toml:36`, then `:35`; upstream #4956 and #5494). At answer time: fuzz no longer regresses FSRS intervals (#4826), set-due-date reads the collection's FSRS flag (#4792), FSRS data is recomputed after a deck change (#5339), and the FSRS flags are cached in the card queues (#4755). | The day set's meaning does not change. The code that decides which new cards are queued, their order and the new count is byte-identical: `scheduler/queue/builder/gathering.rs`, `decks/limits.rs`, `decks/current.rs`, `decks/name.rs`, `scheduler/timing.rs`, `scheduler/new.rs` and `scheduler/queue/{main,entry,learning}.rs`. `builder/mod.rs` only caches one more FSRS flag (`:126`, `:183`, `:227`); `builder/sorting.rs:20` swaps `sort_by` for `sort_by_key`, both stable sorts; `queue/mod.rs:121` computes the top card's next states from the card in hand. The FSRS changes act when a card is answered, which DeckStreak never does. |
| the collection schema | Nothing. `rslib/src/storage/upgrades/mod.rs:5,7,9` holds `SCHEMA_MIN_VERSION` 11, `SCHEMA_STARTING_VERSION` 11 and `SCHEMA_MAX_VERSION` 18 at both tags, and `upgrades/` is byte-identical. | A collection written at `26.05` (schema 18) opens without an upgrade (`storage/sqlite.rs:509`, then `:505`: `let upgrade = ver != SCHEMA_MAX_VERSION;`). The schema-modified stamp does not move, so the upgrade itself never makes a later normal sync demand a full one. |
| the sync protocol | Nothing. `rslib/src/sync/version.rs` is byte-identical: `SYNC_VERSION_MIN` 8 and `SYNC_VERSION_MAX` 11 (`:11-12`). So are `sync/login.rs`, `sync/http_client/`, `sync/http_server/`, `sync/request/` and every file of `sync/collection/` but `status.rs`, where the offline status now reports no changes for a collection that never synced (#5112). Media sync fixed a race (#4635). | A 26.09.3 client needs the same server a 26.05 client does: one that accepts protocol 11 (§8). `crates/ingest/src/engine.rs` calls `normal_sync` (`:191`) and `full_download` (`:215`), never the offline status, and never syncs media (SPEC-022 R5). |
| the build | protoc stays 31.1, same archive and digest (`build/ninja_gen/src/protobuf.rs:24-25` at both tags). The declared MSRV stays `rust-version = "1.80"` (`Cargo.toml:6`). Anki's own `rust-toolchain.toml:3` moves 1.92.0 to 1.97.1 (upstream #5316, a routine bump with clippy-driven edits). `rusqlite` stays 0.36.0 (`Cargo.toml:117`, then `:116`), and `percent-encoding-iri` keeps `ankitects/rust-url` at the same revision (`:27-29`, then `:26-28`). | DeckStreak's protoc pin does not move. Anki's toolchain file applies only inside Anki's own tree; a dependency is compiled by DeckStreak's pinned 1.97.0. The lockfile keeps one `libsqlite3-sys`, 0.34.0, which sqlx 0.9 accepts (measured below). No new git source enters. |
| licences and advisories | `fsrs` 6 drops `burn`: Anki's own lockfile goes from 783 to 596 packages, its `.deny.toml` drops the `paste` and `bincode` exceptions, and its `cargo/licenses.json` gains no licence expression while losing `Unlicense` and `CC0-1.0`. | Measured in a scratch resolution of DeckStreak at the tag: 685 packages become 477 (216 removed, 8 added: `fsrs` 6.6.2, `itertools` 0.15.0, `ndarray` 0.17.2, `priority-queue` 2.7.0, `snafu` and `snafu-derive` 0.9.2, `strum` and `strum_macros` 0.28.0). `cargo deny` passes, and reports RUSTSEC-2024-0436 (`paste`) and RUSTSEC-2025-0141 (`bincode`) as no longer encountered. With `Unlicense` removed from the allow list it still passes: every crate left under that licence also offers MIT. |
| the Python package the predecessor runs | 26.09 removed the legacy `anki.importing` and `anki.exporting` modules. The package's own dependency list is unchanged. | The predecessor imports neither (predecessor `27ee2bc`: only `sync.py` and `pipeline_layers/preread.py` import the engine, lazily). Its plan is private (#234). |

- **The parity goldens do not move, measured.** The registry modules that reach predecessor code
  which calls the engine are `spec_022.py` (through `pipeline.py` and `sync.py`) and `spec_027.py`
  (through `pipeline_layers/ops.py`, which imports the preread layer and `sync.py`). Both import the
  engine lazily, inside functions their adapters never call. Regenerated with Anki's Python package
  at 26.9.3 installed beside the predecessor at `27ee2bc`, all 16 goldens are byte-identical to the
  committed ones, each under the interpreter its own note records, and the generator never imports
  the engine.
- **The pin, measured.** A `[patch."https://github.com/ankitects/anki.git"]` entry that points at a
  commit of the fork, and a direct `git` dependency on that commit, produce the same `Cargo.lock`:
  both differ from the plain 26.09.3 tag only in the `source` of the engine's five packages. The
  direct form breaks the two readers of the dependency line, SPEC-022's A1 check
  (`test_engine_spike_record.py`) and `engine-measure.yml:104`, which both read the upstream tag
  from it. Under `[patch]`, cargo still fetches the upstream repository although nothing in the
  graph comes from it, and an upstream entry left in `allow-git` raises cargo-deny's unmatched-source
  warning. ADR-058 chooses the `[patch]` entry, with `allow-git` naming the fork and `rust-url`.
- **AnkiWeb.** DeckStreak never logs in to AnkiWeb; it syncs from the owner's own sync server
  (ADR-009). AnkiWeb's terms do not allow third-party clients: "AnkiWeb does not currently allow
  access from browser extensions or other third-party clients" (§9).

**Order.** After this plan is accepted, and after the maintainer has pushed the fork's branch and
tagged the pinned commit (#233). The delivery merges in the same window as the predecessor's move
(#234).

## 2. Requirements

R1. The engine's workspace dependency names upstream tag `26.09.3`
    (`anki = { git = "https://github.com/ankitects/anki.git", tag = "26.09.3", features = ["rustls"] }`),
    and the root manifest patches it: `[patch."https://github.com/ankitects/anki.git"]` holds
    `anki = { git = "https://github.com/RexRenatus/anki.git", rev = "<the pinned commit>" }`, the
    fork's commit that is the upstream tag plus exactly the rebuild fix (ADR-058). The patch's
    comment names ADR-058 and #233. Every engine package in `Cargo.lock` comes from that commit, and
    the lockfile otherwise equals the plain 26.09.3 resolution.
R2. The pinned commit carries the fix: with nothing changed, a second
    `cargo build --locked -p deck-streak-ingest` compiles no unit and reports no `Dirty` unit, and
    cargo's own `Finished` time for it is at most 10 seconds.
R3. `deny.toml`'s `allow-git` names exactly `https://github.com/RexRenatus/anki.git` and
    `https://github.com/ankitects/rust-url.git`, and `unknown-git` still refuses any other source.
    The `paste` (RUSTSEC-2024-0436) and `bincode` (RUSTSEC-2025-0141) exceptions and the `Unlicense`
    allowance are removed with their comments, because nothing in the graph needs them at 26.09.3.
    `CC0-1.0` stays: it predates the engine. Every advisory exception that remains is one
    `cargo deny` still encounters, with its reason re-read at the new tag.
R4. The protoc pin does not move: 31.1 with ADR-022's archive digest in every job that compiles
    Rust, in `engine-measure.yml` and in the gate's toolchain stage, because the engine's own build
    pins the same at 26.09.3.
R5. `rust-toolchain.toml` stays at 1.97.0. If the engine fails to build with it, the delivery stops
    and records the failure; a toolchain change is its own decision, never a way to make something
    compile.
R6. ADR-022's protocol runs again at the new pin, unchanged: the same synthetic collection, the same
    budgets, the cold build in `engine-measure.yml` on the delivery's pull request. ADR-009's
    Confirmation records each number with its run (R12). ADR-058's Confirmation records the pinned
    commit, the no-op build before and after in cargo's own time, and CI's warm path after the first
    push to `dev` saves a cache. A budget that fails stops the delivery (ADR-022).
R7. Every SPEC-022 criterion passes at the new pin: A1 to A17, including the budget tests (A2, A3)
    and the no-upload census (A15, A16).
R8. The parity goldens stay byte-identical. A golden that changes is explained by a named upstream
    change before the delivery merges.
R9. `crates/ingest/src/engine.rs`'s adapter keeps every engine type inside `ingest` (SPEC-022 R1).
    Its documentation names the upstream tag and the fork's revision that patches it. Any change
    the engine's API forces stays in that file.
R10. The predecessor moves to the same release in the same window, as the maintainer's act (#234).
    Its plan is private. The delivery is not merged until the maintainer reports that move.
R11. The measured warm path goes to SPEC-038's amendment (#207), with its run ids.
R12. The engine's record follows the pin. SPEC-022's A1 check (`test_engine_spike_record.py`)
    requires ADR-009's Confirmation to name the tag the dependency line pins, so at 26.09.3 it is red
    by construction until the record names the new tag (measured). ADR-009 is accepted and no
    existing line of it changes: the delivery appends to its Confirmation, below the spike's record,
    ADR-022's numbers at 26.09.3 on the pinned commit, with the `engine-measure.yml` run and a table
    of the same shape. The check judges that section's latest table against the budgets, so it
    judges the numbers of the engine that runs.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the engine's dependency names upstream tag 26.09.3, the root manifest patches it by `rev` to a commit of the fork, every engine package in the lockfile comes from that commit, and `allow-git` names exactly the fork and `rust-url` | `test_engine_pin.py` `the_engine_is_patched_by_rev_to_a_commit_of_the_fork` |
| A2 | a second build of `deck-streak-ingest` with nothing changed compiles no unit, within cargo's own time bound | `test_engine_pin.py` `a_second_build_of_ingest_recompiles_nothing` |
| A3 | every advisory exception in `deny.toml` is one `cargo deny` still encounters, and every allowed git source is in the graph | `test_engine_pin.py` `every_advisory_exception_and_git_source_in_deny_toml_is_live` |
| A4 | every job that compiles Rust still installs protoc 31.1 at ADR-022's digest | `test_ci_workflows.py` `every_job_that_compiles_rust_installs_the_pinned_protoc_first` (SPEC-038) |
| A5 | ADR-058 records the pinned commit and its one-file difference from the upstream tag, the no-op build before and after, CI's warm path with its run, and a final status | `test_engine_pin.py` `adr_058_records_the_pinned_commit_and_what_it_saves` |
| A6 | every committed golden is current and well formed | `test_goldens.py` `every_committed_golden_is_current_and_well_formed` (SPEC-029) |
| A7 | SPEC-022's A1: ADR-009's Confirmation names the pinned tag, and its latest record holds ADR-022's budgets, with a final status (R12) | `test_engine_spike_record.py` |
| A8 | SPEC-022's A2: opening the large synthetic collection and resolving its new-card queue stays inside the memory budget | `engine_budget` test |
| A9 | SPEC-022's A3: a full download of the large synthetic collection stays inside the memory budget | `engine_budget` test |
| A10 | SPEC-022's A4: a sync pulls a review made on another client | `sync` test |
| A11 | SPEC-022's A5: a second sync with no change pulls nothing | `sync` test |
| A12 | SPEC-022's A6: a full-sync demand downloads and never uploads | `sync` test |
| A13 | SPEC-022's A7: a server with no collection is refused with `full_upload_required` | `sync` test |
| A14 | SPEC-022's A8: the retry schedule matches the predecessor's golden | `retry` test |
| A15 | SPEC-022's A9: the sync constants equal the predecessor's | `retry` test |
| A16 | SPEC-022's A10: an attempt past its timeout is recorded as `sync_timeout` | `retry` test |
| A17 | SPEC-022's A11: a failed sync records a bounded reason code and no error text | `retry` test |
| A18 | SPEC-022's A12: a second sync waits for the collection lock | `lock` test |
| A19 | SPEC-022's A13: the ingest port declares `sync_runs` exported and erased | `data_rights` test |
| A20 | SPEC-022's A14: a missing sync endpoint refuses start by name | `settings` test |
| A21 | SPEC-022's A15: the recording server sees no upload and no local change in any scenario | `sync` test |
| A22 | SPEC-022's A16: a second scheduled sync in one study day is refused | `sync` test |
| A23 | SPEC-022's A17: an owner trigger within five minutes of a success returns it without syncing | `sync` test |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k the_engine_is_patched_by_rev_to_a_commit_of_the_fork
A2: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k a_second_build_of_ingest_recompiles_nothing
A3: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k every_advisory_exception_and_git_source_in_deny_toml_is_live
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_job_that_compiles_rust_installs_the_pinned_protoc_first
A5: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k adr_058_records_the_pinned_commit_and_what_it_saves
A6: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k every_committed_golden_is_current_and_well_formed
A7: python3 -m unittest discover -s scripts/tests -p test_engine_spike_record.py -k adr_009_records_the_measured_numbers_and_a_final_status
A8: cargo test -p deck-streak-ingest --test engine_budget -- --exact opening_a_large_synthetic_collection_and_its_new_card_queue_stays_inside_the_memory_budget
A9: cargo test -p deck-streak-ingest --test engine_budget -- --exact a_full_download_of_the_large_synthetic_collection_stays_inside_the_memory_budget
A10: cargo test -p deck-streak-ingest --test sync -- --exact a_sync_pulls_a_review_made_on_another_client
A11: cargo test -p deck-streak-ingest --test sync -- --exact a_second_sync_with_no_change_pulls_nothing
A12: cargo test -p deck-streak-ingest --test sync -- --exact a_full_sync_demand_downloads_and_never_uploads
A13: cargo test -p deck-streak-ingest --test sync -- --exact a_server_with_no_collection_is_refused_with_full_upload_required
A14: cargo test -p deck-streak-ingest --test retry -- --exact the_retry_schedule_matches_the_predecessors_golden
A15: cargo test -p deck-streak-ingest --test retry -- --exact the_sync_constants_equal_the_predecessors
A16: cargo test -p deck-streak-ingest --test retry -- --exact an_attempt_past_its_timeout_is_recorded_as_sync_timeout
A17: cargo test -p deck-streak-ingest --test retry -- --exact a_failed_sync_records_a_bounded_reason_code_and_no_error_text
A18: cargo test -p deck-streak-ingest --test lock -- --exact a_second_sync_waits_for_the_collection_lock_and_never_overlaps
A19: cargo test -p deck-streak-ingest --test data_rights -- --exact the_ingest_port_declares_sync_runs_exported_and_erased
A20: cargo test -p deck-streak-ingest --test settings -- --exact a_missing_sync_endpoint_refuses_start_by_name
A21: cargo test -p deck-streak-ingest --test sync -- --exact a_sync_run_sends_no_upload_and_no_local_change
A22: cargo test -p deck-streak-ingest --test sync -- --exact a_second_scheduled_sync_in_one_study_day_is_refused
A23: cargo test -p deck-streak-ingest --test sync -- --exact an_owner_trigger_within_five_minutes_of_a_success_returns_it_without_syncing
```

A1, A2, A3 and A5 are new, in `scripts/tests/test_engine_pin.py`. A1 reads the manifest, the
lockfile and `deny.toml`. A2 runs `cargo build --locked -v -p deck-streak-ingest` twice against the
workspace's own target directory, and reads the second run's `Compiling`, `Dirty` and `Finished`
lines. At the base it is red for its reason: the second run recompiles `anki_proto` and `anki`. A3
runs `cargo deny --locked --format json check advisories sources`, and refuses a zero count of
examined exceptions. A5 reads ADR-058's Confirmation. A4, A6 and A8 to A23 exist and must stay green
at the new pin. A7 exists, and it turns green with R12's record. Each budget test runs alone in its
own process, as SPEC-022 §6 requires.

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | workspace | changed: the engine's tag becomes `26.09.3`, and a `[patch]` entry points it at the fork's commit by `rev` (R1) |
| `Cargo.lock` | workspace | changed: the engine's packages from the fork's commit, and the packages the tag moves |
| `deny.toml` | workspace | changed: `allow-git`, two advisory exceptions and the `Unlicense` allowance (R3) |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | changed: its documentation names the patched revision, and any change the engine's API forces (R9) |
| `scripts/tests/test_engine_pin.py` | repo | added: A1, A2, A3, A5 |
| `scripts/tests/test_engine_spike_record.py` | repo | changed: the Confirmation's latest table is the one judged (R12) |
| `docs/decisions/ADR-009-ingest-from-the-anki-sync-server.md` | repo | changed: its Confirmation gains the record at 26.09.3 below the spike's, with no existing line changed (R12) |
| `.github/workflows/ci.yml` | repo | changed only if A2 cannot run where the gate's python stage runs in CI; the delivery measures both places and records its choice |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | repo | changed: status, the pinned commit and the Confirmation's measurements |
| `docs/specs/planned/SPEC-055-the-engine-moves-to-26-09-3-together-with-the-predecessor.md` | repo | moved to `docs/specs/`, with §7 filled |
| `docs/red-first/SPEC-055.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It submits nothing upstream and creates no fork branch: the upstream pull request, the fork's
  branch and the tag on the pinned commit are the maintainer's acts (#233).
- It does not remove the fork. That is a later delivery, once an upstream release carries the fix
  (#233).
- It does not move the predecessor. The maintainer moves it in the same window, and its plan is
  private (#234).
- It deploys nothing. DeckStreak's first deploy is W2's (#42), behind the owner's gate (#161).
- It changes no sync behaviour: one scheduled sync per study day, the owner's triggers, and never an
  upload stay as ADR-037 decided (#164 owns any cadence change at cutover).
- It adds no CI cache for dependency checkouts. The engine's second rebuild watch is inert for a git
  dependency, so such a cache would buy nothing (#228).
- It does not amend SPEC-038. It hands the re-measured warm path to SPEC-038's amendment (#207).
- It changes nothing on the owner's devices or sync server. §8 lists what the owner checks, and host
  findings stay private (#167).

## 6. Risks

- **The pinned commit becomes unreachable.** A force-push or a deleted branch on the fork could let
  the host drop the commit, and a fresh CI checkout would then fail to fetch it. CI's fetch fails by
  name. The mitigation is a tag on the pinned commit and a branch that is never force-pushed while
  it is pinned (ADR-058, #233).
- **The fork carries more than the fix.** The delivery records `git diff --stat 26.09.3 <rev>` on
  the fork in ADR-058's Confirmation: one file, `rslib/io/src/lib.rs`. A2 proves the fix's effect.
- **The patch is dropped and nobody notices.** Without the `[patch]` entry the engine resolves from
  upstream, which `allow-git` no longer names, so the audit refuses it by name, and A1 fails on the
  lockfile's sources.
- **A fresh checkout fetches two repositories.** Cargo fetches upstream to resolve the patch as well
  as the fork (measured). CI's Rust jobs already cache cargo's git database (`ci.yml`), which holds
  both; the delivery's cold and warm runs show what the second fetch costs.
- **The engine's API changed under ingest.** The build fails at the new pin, and the adapter keeps
  every engine type inside `ingest`, so the change stays in `engine.rs` (R9). The measurement table
  records whether the build needed one.
- **The engine needs a newer toolchain than 1.97.0.** The build fails, and the delivery stops (R5).
  Anki's move to 1.97.1 was a routine bump, and its declared MSRV is 1.80.
- **A later sync demands a full sync.** Not because of the upgrade: the schema and the protocol are
  unchanged (§1). If the owner's own client forces one, DeckStreak downloads and never uploads
  (SPEC-022 R6, A12, A21).
- **A removed exception comes back.** If `paste` or `bincode` re-enter the graph, the audit fails on
  their advisories again. That failure is the point of removing a stale exception rather than
  keeping it.
- **The goldens drift.** A6 keeps every golden current, and §1's regeneration found none changed.

## 7. Measurements

ADR-022's protocol at 26.05 comes from `engine-measure.yml` run 36357990387 (SPEC-022 §1). At
26.09.3 it comes from `engine-measure.yml` run 36374499584, from a measurement pull request that
changed only the engine's tag and `Cargo.lock` and was closed unmerged. Both ran on GitHub-hosted
runners, whose build time varies from run to run, so a single cold-build sample is read as a range
and only the deterministic outputs compare exactly. The local numbers were measured on the
maintainer's machine, in cargo's own `Finished` time. The delivery re-measures on the fork's commit
(R6).

| measure | budget (ADR-022) | 26.05 | 26.09.3 | 26.09.3 with the fix |
|---|---|---|---|---|
| cold build, the protoc download included | at most 20 minutes | 4.7 minutes (282 s; 197 to 301 s over nine runs) | 4.2 minutes (249 s; 214 s in a second run) | the delivery measures it (R6) |
| the stripped `engine_probe` | at most 100 MiB | 20.6 MiB (21,661,952 bytes) | 19.3 MiB (20,236,032 bytes, 6.6 % smaller) | the delivery measures it (R6) |
| peak RSS, open and queue | at most 256 MiB | 29.4 MiB | 28.8 MiB | the delivery measures it (R6) |
| peak RSS, full download | at most 256 MiB | 234.0 MiB | 232.1 to 232.2 MiB (90.7 % of the budget) | the delivery measures it (R6) |
| incremental sync of 100 reviews | at most 60 seconds | 0.12 seconds | 0.12 seconds | the delivery measures it (R6) |
| `cargo deny check licenses` | pass | pass | pass | pass (a scratch resolution, §1) |
| SPEC-022's A1 to A17, locally | all pass | all pass | A2 to A17 pass (16 of 16), the census A15 and A16 included; A1 is red by construction until the record follows the tag (R12) | the delivery runs them (R7) |
| the parity oracle's tests | all pass | 20 pass, 16 goldens | 20 pass, the 16 goldens unchanged | unchanged (§1) |
| a cold debug build of `deck-streak-ingest`, locally | none | 69 s, 435 units | 59.5 s, 373 units | not measured |
| a no-op `cargo build -p deck-streak-ingest`, locally | none | 38 to 51 s | 31.4 to 34.5 s, with the same `Dirty` reasons | Fresh in 0.33 to 0.34 s with the first cause neutralised; the drafted fix consumed as a git source took 0.60 s and 0.53 s against 39.70 s and 43.00 s unpatched |
| the lockfile | none | 685 packages; 34 duplicate warnings | 477 packages; one `libsqlite3-sys`, 0.34.0; 31 duplicate warnings, the new ones (`itertools`, `snafu`, `strum`, `rand`) through `fsrs` 6.6.2 | the same, with five engine sources moved (§1) |
| CI's warm path | none | SPEC-038 §7 | not measured: only a push to `dev` saves a cache | the delivery measures it (R6, R11) |

Every budget holds at 26.09.3, and no pin besides the engine's has to move: protoc stays 31.1, the
engine's declared MSRV stays 1.80, and the toolchain stays 1.97.0.

## 8. The owner's and the devices' checklist

The upgrade changes neither the collection schema nor the sync protocol (§1), so no device has to
move with it. The owner checks three versions once, before the window, and one sync after it.

| check | where | must be | why | if it lags |
|---|---|---|---|---|
| the self-hosted sync server's version | the server's own version report | at least Anki 2.1.57, the first release whose built-in server speaks sync protocol 11 | Both engines speak protocol 11 (`rslib/src/sync/version.rs:11-12`). Anki's manual warns that "syncing may stop working if you update your Anki clients without also updating the server": that applies when the protocol changes, and 26.09.3's does not. A server that serves 26.05 today serves 26.09.3. | Below 2.1.57, every sync of DeckStreak and of the predecessor fails; DeckStreak records `server_error`, and three consecutive failures page (SPEC-027). Keep the server as it is during the window, so one thing changes at a time. |
| Anki desktop | Help, then About | any release that syncs with that server today | It writes the collection that everything else reads, at schema 18 and over protocol 11, and 26.09.3 changes neither. 26.09 carries security fixes for the desktop's editor, which the owner may want on their own merits. | Nothing changes for DeckStreak, which never talks to a device. |
| AnkiDroid or AnkiMobile | the app's About screen | any release that syncs with that server today | The same reasons as the desktop. | Nothing changes for DeckStreak. A device that moves to a release with a new sync protocol may stop syncing until the server is updated, per the manual; DeckStreak's pinned engine is not involved. |
| AnkiWeb | none | not used | DeckStreak syncs only from the owner's server, and AnkiWeb's terms do not allow third-party clients. | Not applicable. |
| after the window | the next scheduled sync of whatever syncs from that server with the new engine | a normal sync, not a full one | It shows that the move demanded no full sync (§1). | If any device ever asks for a full sync, the owner chooses its direction on that device. DeckStreak always downloads (ADR-037). |

## 9. References

Each read on 2026-09-28.

- Anki's release notes: https://github.com/ankitects/anki/releases/tag/26.08,
  https://github.com/ankitects/anki/releases/tag/26.08.1,
  https://github.com/ankitects/anki/releases/tag/26.09,
  https://github.com/ankitects/anki/releases/tag/26.09.1,
  https://github.com/ankitects/anki/releases/tag/26.09.2,
  https://github.com/ankitects/anki/releases/tag/26.09.3, and the betas' pages under the same path.
  Change notes since 23.12 live there (`docs-site/releases/changes/changes/github.mdx` at
  `26.09.3`).
- The tag diff: `git diff 26.05 26.09.3` in a read-only clone of https://github.com/ankitects/anki.
- Upstream pull requests cited: https://github.com/ankitects/anki/pull/4755, /4792, /4826, /4956,
  /5112, /5316, /5339, /5494 and /4635.
- The sync server: https://docs.ankiweb.net/sync-server.html ("the sync server built into the
  desktop version of Anki as of version 2.1.57+").
- AnkiWeb's terms: https://ankiweb.net/account/terms.
- Cargo's mtime rule for `$CARGO_HOME`: https://github.com/rust-lang/cargo/pull/11613.
- Cargo's `[patch]` and git dependencies: https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html
  and https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html (Context7
  `/websites/doc_rust-lang_cargo`).
- cargo-deny's sources, advisory and licence checks:
  https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html (Context7
  `/websites/embarkstudios_github_io_cargo-deny`).
