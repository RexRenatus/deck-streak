# SPEC-055: the engine moves to 26.09.3 on a patched fork until upstream carries the rebuild fix

- **Wave:** W0. **Issue:** #235 (epic #1). **Context(s):** `deck-streak-ingest` (the engine pin);
  `repo` (`Cargo.toml`, `Cargo.lock`, `deny.toml`, the engine's measurement).
- **Decided by:** ADR-058 (this SPEC's: the engine pins a patched fork of 26.09.3 until upstream
  carries the fix), ADR-022 (the spike's protocol and budgets, whose pin rule ADR-058 amends),
  ADR-009 (the engine), ADR-037 (never upload), ADR-012 (the parity oracle), ADR-018 (licences) and
  ADR-055 (CI's jobs and caches).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-055.md`, and the measurement
  that accepted ADR-058 (`engine-measure.yml` run 36386849300). The delivery amended R6, R10, A5,
  §3's account of A3, §5, §6 and the manifest's `ci.yml` row, each for the reason §7 gives.

## 1. The problem, measured

- **The decision.** The owner decided to move Anki's engine from tag `26.05` to `26.09.3`. The
  engine's rebuild on every cargo command (#228) is fixed two ways at once: the maintainer submits
  the fix upstream, and DeckStreak carries it on a fork until an upstream release holds it
  (ADR-058).
- **The releases in range.** Anki published no 26.06 or 26.07. The range is 26.08b1, 26.08b2, 26.08,
  26.08.1, 26.09b1 to 26.09b3, 26.09, 26.09.1 (skipped for a Windows packaging issue), 26.09.2 and
  26.09.3, whose notes are on GitHub's release pages (§8). The tree is compared directly: `26.05` is
  `e64c6b1`, `26.09.3` is `29bb700`, and neither is an ancestor of the other, because Anki tags each
  release on its own branch. `git diff --stat 26.05 26.09.3 -- rslib/` touches 82 files with 7,556
  lines added and 384 removed, and most of the added lines are new service-layer tests.
- **The rebuild is still there.** The cause #228 names is byte-identical at both tags and on
  upstream `main` (`1f7c8d7c4`): `rslib/io/src/lib.rs:353-355` registers every file the `anki_proto`
  build script writes, and `rslib/proto_gen/src/lib.rs:253` writes the prost output a second time,
  so its mtime always postdates the run. The second watch, `rslib/build.rs:13`, is inert for a git
  dependency, because cargo skips mtime checks for paths under `$CARGO_HOME` (rust-lang/cargo#11613).
  A pin bump alone therefore saves nothing: measured on the maintainer's machine, a no-op
  `cargo build -p deck-streak-ingest` takes 31 to 35 s at 26.09.3, with the same `Dirty` reasons as
  at 26.05, and every unit is Fresh in 0.33 s once that cause is neutralised (§7). The fix is one
  commit that changes only `rslib/io/src/lib.rs`, and it applies unchanged to `26.09.3`
  (`git apply --check`).

What the tag diff changes, by the areas DeckStreak depends on (file:line at `26.05`, then at
`26.09.3`):

| area | what changed | what it means here |
|---|---|---|
| scheduling and FSRS | `fsrs` 5.2.0 becomes 6.6.2 (`Cargo.toml:36`, then `:35`; ankitects/anki#4956 and ankitects/anki#5494). At answer time: fuzz no longer regresses FSRS intervals (ankitects/anki#4826), set-due-date reads the collection's FSRS flag (ankitects/anki#4792), FSRS data is recomputed after a deck change (ankitects/anki#5339), and the FSRS flags are cached in the card queues (ankitects/anki#4755). | The day set's meaning does not change. The code that decides which new cards are queued, their order and the new count is byte-identical: `scheduler/queue/builder/gathering.rs`, `decks/limits.rs`, `decks/current.rs`, `decks/name.rs`, `scheduler/timing.rs`, `scheduler/new.rs` and `scheduler/queue/{main,entry,learning}.rs`. `builder/mod.rs` only caches one more FSRS flag (`:126`, `:183`, `:227`); `builder/sorting.rs:20` swaps `sort_by` for `sort_by_key`, both stable sorts; `queue/mod.rs:121` computes the top card's next states from the card in hand. The FSRS changes act when a card is answered, which DeckStreak never does. |
| the collection schema | Nothing. `rslib/src/storage/upgrades/mod.rs:5,7,9` holds `SCHEMA_MIN_VERSION` 11, `SCHEMA_STARTING_VERSION` 11 and `SCHEMA_MAX_VERSION` 18 at both tags, and `upgrades/` is byte-identical. | A collection written at `26.05` (schema 18) opens without an upgrade (`storage/sqlite.rs:509`, then `:505`: `let upgrade = ver != SCHEMA_MAX_VERSION;`). The schema-modified stamp does not move, so the upgrade itself never makes a later normal sync demand a full one. |
| the sync protocol | Nothing. `rslib/src/sync/version.rs` is byte-identical: `SYNC_VERSION_MIN` 8 and `SYNC_VERSION_MAX` 11 (`:11-12`). So are `sync/login.rs`, `sync/http_client/`, `sync/http_server/`, `sync/request/` and every file of `sync/collection/` but `status.rs`, where the offline status now reports no changes for a collection that never synced (ankitects/anki#5112), and `tests.rs`, which is compiled only for tests. Media sync fixed a race (ankitects/anki#4635). | A 26.09.3 client needs the same server a 26.05 client does: one that accepts protocol 11, which Anki's built-in server does from 2.1.57 (§8). `crates/ingest/src/engine.rs` calls `normal_sync` (`:191`) and `full_download` (`:215`), never the offline status, and never syncs media (SPEC-022 R5). |
| the build | protoc stays 31.1, same archive and digest (`build/ninja_gen/src/protobuf.rs:24-25` at both tags). The declared MSRV stays `rust-version = "1.80"` (`Cargo.toml:6`). Anki's own `rust-toolchain.toml:3` moves 1.92.0 to 1.97.1 (ankitects/anki#5316, a routine bump with clippy-driven edits). `rusqlite` stays 0.36.0 (`Cargo.toml:117`, then `:116`), and `percent-encoding-iri` keeps `ankitects/rust-url` at the same revision (`:27-29`, then `:26-28`). | DeckStreak's protoc pin does not move. Anki's toolchain file applies only inside Anki's own tree; a dependency is compiled by DeckStreak's pinned 1.97.0. The lockfile keeps one `libsqlite3-sys`, 0.34.0, which sqlx 0.9 accepts (measured below). No new git source enters. |
| licences and advisories | `fsrs` 6 drops `burn`: Anki's own lockfile goes from 783 to 596 packages, its `.deny.toml` drops the `paste` and `bincode` exceptions, and its `cargo/licenses.json` gains no licence expression while losing `Unlicense` and `CC0-1.0`. | Measured in a scratch resolution of DeckStreak at the tag: 685 packages become 477 (216 removed, 8 added: `fsrs` 6.6.2, `itertools` 0.15.0, `ndarray` 0.17.2, `priority-queue` 2.7.0, `snafu` and `snafu-derive` 0.9.2, `strum` and `strum_macros` 0.28.0). `cargo deny` passes, and reports RUSTSEC-2024-0436 (`paste`) and RUSTSEC-2025-0141 (`bincode`) as no longer encountered. With `Unlicense` removed from the allow list it still passes: every crate left under that licence also offers MIT. |

- **The parity goldens do not move, measured.** The generator never imports the engine: after a
  full regeneration no `anki` module is loaded, so no engine version can move a golden. Every golden
  regenerates byte-identical to the committed one, under the interpreter its own note records.
- **The pin, measured.** A `[patch."https://github.com/ankitects/anki.git"]` entry that points at a
  commit of the fork, and a direct `git` dependency on that commit, produce the same `Cargo.lock`:
  both differ from the plain 26.09.3 tag only in the `source` of the engine's five packages. The
  direct form breaks the two readers of the dependency line, SPEC-022's A1 check
  (`test_engine_spike_record.py`) and `engine-measure.yml:104`, which both read the upstream tag
  from it. Under `[patch]`, cargo loads the upstream repository only when it resolves the patch
  anew, never with a committed `Cargo.lock` (§6), and nothing in the graph comes from it, so an
  upstream entry left in `allow-git` raises cargo-deny's unmatched-source warning. ADR-058 chooses
  the `[patch]` entry, with `allow-git` naming the fork and `rust-url`.
- **AnkiWeb.** DeckStreak never logs in to AnkiWeb; it syncs from a self-hosted sync server
  (ADR-009). AnkiWeb's terms do not allow third-party clients: "AnkiWeb does not currently allow
  access from browser extensions or other third-party clients" (§8).

**Order.** After this plan is accepted, and after the maintainer has pushed the fork's branch and
tagged the pinned commit (#233).

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
    (amended by ADR-058's amendment, SPEC-342)
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
    Confirmation records each number with its run (R11). ADR-058's Confirmation records the pinned
    commit, the no-op build before and after in cargo's own time, and CI's numbers on the delivery's
    pull request: the fallback restore, the jobs' times by SPEC-038 §8's metric, and the
    `engine-measure.yml` run. CI's warm path after the first push to `dev` that saves a cache is
    appended to ADR-058's Confirmation by the orchestrator after the merge, because only that push
    saves a cache. A budget that fails stops the delivery (ADR-022).
R7. Every SPEC-022 criterion passes at the new pin: A1 to A17, including the budget tests (A2, A3)
    and the no-upload census (A15, A16).
R8. The parity goldens stay byte-identical. A golden that changes is explained by a named upstream
    change before the delivery merges.
R9. `crates/ingest/src/engine.rs`'s adapter keeps every engine type inside `ingest` (SPEC-022 R1).
    Its documentation names the upstream tag and the fork's revision that patches it. Any change
    the engine's API forces stays in that file.
R10. The warm path after the first push to `dev` that saves a cache is appended, with its run ids,
    to ADR-058's Confirmation by the orchestrator after the merge. SPEC-038's amendment (#207) is
    closed, and a pull request that changes `Cargo.lock` restores `dev`'s entry by its fallback key,
    so the delivery cannot measure that path itself.
R11. The engine's record follows the pin. SPEC-022's A1 check (`test_engine_spike_record.py`)
    requires ADR-009's Confirmation to name the tag the dependency line pins, so at 26.09.3 it is red
    by construction until the record names the new tag (measured). ADR-009 is accepted and no
    existing line of it changes: the delivery appends to its Confirmation, below the spike's record,
    ADR-022's numbers at 26.09.3 on the pinned commit, with the `engine-measure.yml` run and a table
    of the same shape. The check judges that section's latest table against the budgets, so it
    judges the numbers of the engine that runs.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the engine's dependency names upstream tag 26.09.3, the root manifest patches it by `rev` to a commit of the fork, every engine package in the lockfile comes from that commit, and `allow-git` names exactly the fork and `rust-url` (amended by ADR-058's amendment, SPEC-342) | `test_engine_pin.py` `the_engine_is_patched_by_rev_to_a_commit_of_the_fork` |
| A2 | a second build of `deck-streak-ingest` with nothing changed compiles no unit, within cargo's own time bound | `test_engine_pin.py` `a_second_build_of_ingest_recompiles_nothing` |
| A3 | every advisory exception in `deny.toml` is one `cargo deny` still encounters, and every allowed git source is in the graph | `test_engine_pin.py` `every_advisory_exception_and_git_source_in_deny_toml_is_live` |
| A4 | every job that compiles Rust still installs protoc 31.1 at ADR-022's digest | `test_ci_workflows.py` `every_job_that_compiles_rust_installs_the_pinned_protoc_first` (SPEC-038) |
| A5 | ADR-058 records the pinned commit and its one-file difference from the upstream tag, the no-op build before and after, CI's numbers on the delivery's pull request with their runs, and a final status | `test_engine_pin.py` `adr_058_records_the_pinned_commit_and_what_it_saves` |
| A6 | every committed golden is current and well formed | `test_goldens.py` `every_committed_golden_is_current_and_well_formed` (SPEC-029) |
| A7 | SPEC-022's A1: ADR-009's Confirmation names the pinned tag, and its latest record holds ADR-022's budgets, with a final status (R11) | `test_engine_spike_record.py` |
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
| A24 | `deny.toml` holds no `[advisories]` ignore entry in either form, and its `unmaintained` scope is `"all"`; each planted defect reads one finding | `test_engine_pin.py` `deny_toml_admits_no_advisory_exception_and_no_narrowed_scope` |
| A25 | no unmaintained crate this SPEC's amendments retire is a package of `Cargo.lock`; a planted lock holding each reads one finding per crate, and a clean planted lock none | `test_engine_pin.py` `the_retired_unmaintained_crates_are_absent_from_the_lockfile` |

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
A24: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k deny_toml_admits_no_advisory_exception_and_no_narrowed_scope
A25: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k the_retired_unmaintained_crates_are_absent_from_the_lockfile
```

A1, A2, A3 and A5 are new, in `scripts/tests/test_engine_pin.py`. A1 reads the manifest, the
lockfile and `deny.toml`. A2 runs `cargo build --locked -v -p deck-streak-ingest` twice against the
workspace's own target directory, and reads the second run's `Compiling`, `Dirty` and `Finished`
lines. At the base it is red for its reason: the second run recompiles `anki_proto` and `anki`. A3
runs `cargo deny --locked --log-level info --format json check advisories sources`, whose notes
name each exception it encounters and each crate a source allowance admits, and refuses a zero
count of examined exceptions. A5 reads ADR-058's Confirmation. A4, A6 and A8 to A23 exist and must
stay green at the new pin. A7 exists, and it turns green with R11's record. Each budget test runs
alone in its own process, as SPEC-022 §6 requires.

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | workspace | changed: the engine's tag becomes `26.09.3`, and a `[patch]` entry points it at the fork's commit by `rev` (R1) |
| `Cargo.lock` | workspace | changed: the engine's packages from the fork's commit, and the packages the tag moves |
| `deny.toml` | workspace | changed: `allow-git`, two advisory exceptions and the `Unlicense` allowance (R3) |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | changed: its documentation names the patched revision, and any change the engine's API forces (R9) |
| `scripts/tests/test_engine_pin.py` | repo | added: A1, A2, A3, A5 |
| `scripts/tests/test_engine_spike_record.py` | repo | changed: the Confirmation's latest table is the one judged (R11) |
| `docs/decisions/ADR-009-ingest-from-the-anki-sync-server.md` | repo | changed: its Confirmation gains the record at 26.09.3 below the spike's, with no existing line changed (R11) |
| `.github/workflows/ci.yml` | repo | changed: the `hygiene` job, which runs the gate's python stage, installs cargo-deny, because A3 runs `cargo deny` there. A2 runs there as the job stood, measured in both places, locally and in CI (§7) |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | repo | changed: status, the pinned commit and the Confirmation's measurements |
| `docs/specs/planned/SPEC-055-the-engine-moves-to-26-09-3-on-a-patched-fork.md` | repo | moved to `docs/specs/`, with §7 filled |
| `docs/red-first/SPEC-055.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It submits nothing upstream and creates no fork branch: the upstream pull request, the fork's
  branch and the tag on the pinned commit are the maintainer's acts (#233).
- It does not remove the fork. That is a later delivery, once an upstream release carries the fix
  (#233).
- It deploys nothing. DeckStreak's first deploy is W2's (#42), behind the owner's gate (#161).
- It changes no sync behaviour: one scheduled sync per study day, the owner's triggers, and never an
  upload stay as ADR-037 decided (#164 owns any cadence change at cutover).
- It adds no CI cache for dependency checkouts. The engine's second rebuild watch is inert for a git
  dependency, so such a cache would buy nothing (#228).
- It does not amend SPEC-038, whose amendment is closed (#207). CI's warm path after the first push
  to `dev` is appended to ADR-058's Confirmation by the orchestrator after the merge (R10).
- It changes nothing on the owner's devices or sync server (#167).

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
- **Moving the pin needs upstream reachable.** Cargo loads the upstream repository only when it
  resolves the patch anew (a tag or `rev` bump, or `cargo update`), so a pin move fails by name
  while upstream is unreachable. With the committed `Cargo.lock` it never loads it: measured
  offline, with the dependency and the patch pointed at an upstream URL that was never fetched, a
  `--locked` resolution succeeds, and the same resolution without the lock fails for want of
  upstream. The gate runs every cargo command `--locked`, so a fresh CI checkout fetches the fork,
  with the submodules it names, and `ankitects/rust-url`, and never upstream.
- **The engine's API changed under ingest.** The build fails at the new pin, and the adapter keeps
  every engine type inside `ingest`, so the change stays in `engine.rs` (R9). The measurement table
  records whether the build needed one.
- **The engine needs a newer toolchain than 1.97.0.** The build fails, and the delivery stops (R5).
  Anki's move to 1.97.1 was a routine bump, and its declared MSRV is 1.80.
- **A later sync demands a full sync.** Not because of the upgrade: the schema and the protocol are
  unchanged (§1). If a client forces one, DeckStreak downloads and never uploads (SPEC-022 R6, A12,
  A21).
- **A removed exception comes back.** If `paste` or `bincode` re-enter the graph, the audit fails on
  their advisories again. That failure is the point of removing a stale exception rather than
  keeping it.
- **The goldens drift.** A6 keeps every golden current, and §1's regeneration found none changed.

## 7. Measurements

ADR-022's protocol at 26.05 comes from `engine-measure.yml` run 36357990387 (SPEC-022 §1), except
where a cell names another run. At 26.09.3 it comes from `engine-measure.yml` run 36374499584, from
a measurement pull request that changed only the engine's tag and `Cargo.lock` and was closed
unmerged. Both ran on GitHub-hosted runners, whose build time varies from run to run, so a single
cold-build sample is read as a range and only the deterministic outputs compare exactly. The local
numbers were measured on the maintainer's machine, in cargo's own `Finished` time. The delivery
re-measures on the fork's commit (R6): its last column comes from
`engine-measure.yml` run 36386849300 on this delivery's pull request, at `4df16b0`, whose engine is
the pinned commit.

| measure | budget (ADR-022) | 26.05 | 26.09.3 | 26.09.3 with the fix |
|---|---|---|---|---|
| cold build, the protoc download included | at most 20 minutes | 4.7 minutes (282 s; 197 to 301 s over nine runs) | 4.2 minutes (249 s; 214 s in a second run) | 4.4 minutes (262 s, `engine-measure.yml` run 36386849300 at `4df16b0`) |
| the stripped `engine_probe` | at most 100 MiB | 20.7 MiB (21,661,952 bytes, run 36371774969 at `f4c0111`) | 19.3 MiB (20,236,032 bytes, 6.6 % smaller) | 19.3 MiB (20,234,656 bytes) |
| peak RSS, open and queue | at most 256 MiB | 29.4 MiB | 28.8 MiB | 28.7 MiB |
| peak RSS, full download | at most 256 MiB | 234.0 MiB | 232.1 to 232.2 MiB (90.7 % of the budget) | 232.2 MiB (90.7 % of the budget) |
| incremental sync of 100 reviews | at most 60 seconds | 0.12 seconds | 0.12 seconds | 0.14 seconds |
| `cargo deny check licenses` | pass | pass | pass | pass, with `Unlicense` gone from the allow list (run 36386849300, and the gate's `audit-rust`) |
| SPEC-022's A1 to A17, locally | all pass | all pass | A2 to A17 pass (16 of 16), the census A15 and A16 included; A1 is red by construction until the record follows the tag (R11) | all pass (17 of 17): A1 once R11's record is written, and A2 to A17 with the census A15 and A16, each budget test alone in its own process |
| the parity oracle's tests | all pass | 20 pass, 16 goldens | 20 pass, the 16 goldens unchanged | 21 pass; all 21 goldens regenerate byte-identical under the interpreters their notes record, and no `anki` module loads |
| a cold debug build of `deck-streak-ingest`, locally | none | 69 s, 435 units | 59.5 s, 373 units | not measured |
| a no-op `cargo build -p deck-streak-ingest`, locally | none | 38 to 51 s | 31.4 to 34.5 s, with the same `Dirty` reasons | Fresh in 0.33 to 0.34 s with the first cause neutralised; the drafted fix consumed as a git source took 0.60 s and 0.53 s against 39.70 s and 43.00 s unpatched. At the pinned commit: 0.33 to 0.35 s over five runs, every unit Fresh (A2 judged 373), against 29.3 to 38.2 s over four runs of 26.09.3 unpatched, measured the same way |
| the lockfile | none | 685 packages; 34 duplicate warnings | 477 packages; one `libsqlite3-sys`, 0.34.0; 31 duplicate warnings, the new ones (`itertools`, `snafu`, `strum`, `rand`) through `fsrs` 6.6.2 | 477 packages and 31 duplicate warnings. A scratch resolution of the plain tag and one with the patch differ only in the `source` of the five engine packages, and the committed lockfile is the second, byte for byte |
| CI's warm path | none | SPEC-038 §7 | not measured: only a push to `dev` saves a cache | not measurable here: this pull request's `ci.yml` run 36386849369 restored `dev`'s entry by its fallback key (ADR-058's Confirmation), and the warm path after the first push to `dev` is appended there by the orchestrator (R6, R10) |

Every budget holds at 26.09.3, and no pin besides the engine's has to move: protoc stays 31.1, the
engine's declared MSRV stays 1.80, and the toolchain stays 1.97.0.

Every budget holds at the pinned commit too, the engine builds with toolchain 1.97.0, and its API
forced no change in `crates/ingest/src/engine.rs`: only the adapter's documentation moved (R9).

**Where A2 runs.** A2 is a guard test of the gate's python stage, and the manifest left its place
in CI open, so it was measured in both places the stage runs:

- **locally**, in the worktree's own target: at the pinned commit the second build judged 373 units
  Fresh and compiled none, where at the base it recompiled `anki_proto`, `anki` and the ingest crate
  in 35.5 s of cargo's own time;
- **in CI**, where the `hygiene` job runs the python stage with the pinned toolchain, protoc and the
  restored Rust cache: in `ci.yml` run 36386849369 its first build took 30.6 s and its second 0.2 s
  of wall time, 373 units Fresh, and the stage took 72 s for 120 tests, against 52 s for 115 tests
  on `dev` (run 36383672922).

It stays there. The `hygiene` job needed only cargo-deny, for A3.

**CI on this pull request.** The pull request changes `Cargo.lock`, so `ci.yml` run 36386849369 at
`4df16b0` restored `dev`'s Rust entry by its fallback key, and every Rust job compiled the fork's
engine, which that entry lacks. The slower gate job, an `engine` leg, took 3m23s, and the gate,
with `ci`'s 4 s, 3m27s. Both are past SPEC-038 R15's warm bounds of 3m15s and 3m20s, which a first
run after a lockfile change cannot meet (ADR-055). The `rust` job's `doctest` stage found every unit
Fresh and took 1 s, against 31 s on `dev` (run 36383672922), where each cargo command recompiled
the engine. The warm path after the merge is appended to ADR-058 by the orchestrator (R6, R10).

### Amended at delivery

Each of these is corrected above, for the reason given.

- **R6, R10 and A5: CI's warm path after the merge.** Only a push to `dev` saves a cache (ADR-055),
  and a pull request that changes `Cargo.lock` restores `dev`'s entry by its fallback key, so the
  delivery cannot measure the warm path after its own merge, and SPEC-038's amendment (#207) is
  closed. ADR-058's Confirmation records the pull request's own CI numbers (the fallback restore,
  SPEC-038 §8's gate time and the `engine-measure.yml` run), and the orchestrator appends the warm
  path after the first push to `dev` that saves a cache. A5's criterion names what the pull request
  can record, and §5's bullet on SPEC-038 follows.
- **§3: A3 reads the audit at the info level.** At its default level cargo deny prints only
  warnings, so an encountered exception would show as the absence of `advisory-not-detected`.
  Measured, the notes that name each exception it encounters (`advisory-ignored`) and each crate a
  source allowance admits (`allowed-source`) print only with `--log-level info`, and A3 reads them
  as positive evidence.
- **The manifest's `ci.yml` row.** A3 runs `cargo deny` in the gate's python stage, which CI runs
  in the `hygiene` job, and that job had no cargo-deny. It installs it with the action and commit
  the `rust` job already pins; A2 needed nothing more (above).
- **§6: "If a client forces one".** The risk holds for any client, not one in particular.

## 8. References

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

## 9. Amendment: the paste and basic-toml exceptions leave deny.toml (#1001)

### Problem

`deny.toml` carried eight `[advisories]` ignore entries, each an unmaintained notice with no patched
release. Six reach the workspace only through the engine (#256). The other two come through crates
this repository does not own: RUSTSEC-2024-0436 (`paste`) and RUSTSEC-2026-0374 (`basic-toml`). An
exception is not a fix. The lockfile census (tomllib over `Cargo.lock`, 656 packages): `paste`
1.0.15 is named by `frankenstein` 0.52.1, the bot's Bot API client (ADR-026), and by
`uniffi_bindgen` 0.29.1, the bindings generator (ADR-345), which declares it and uses it in no
source file; `basic-toml` 0.1.10 is named by `rinja_derive` 0.3.5, which `uniffi_bindgen` reaches
through `rinja`. Neither brings in a package that nothing else names, so these two are all the lock
loses. No release of a parent drops its crate in a form this workspace can take: `frankenstein`'s
default branch still names `paste`, `uniffi` is pinned at `=0.29.1` (ADR-345), and askama replaced
`basic-toml` with `toml` on its main branch with no release yet.

### Requirements

R12. `deny.toml` holds no ignore entry for RUSTSEC-2024-0436 or RUSTSEC-2026-0374, and keeps the
    other six, each one `cargo deny` still encounters (A3).
R13. `paste` and `basic-toml` leave the lockfile by the routes of the table below (A25). No other
    package leaves it or enters it, except that `uniffi_internal_macros`, which `uniffi_meta` names
    by path, comes from the uniffi fork at 0.29.1, the version the 0.29.1 release names, in place
    of the registry's 0.29.5; the lock's diff is read at review.
R14. Each fork is pinned by `rev`, kept fetchable by a named branch and named in `allow-git`, whose
    exact list in `scripts/tests/test_engine_pin.py` grows by the three sources (A1), and each
    entry goes when a release of its parent that this workspace can take drops the crate (ADR-657).
    `uniffi` stays at `=0.29.1` (ADR-345).

| crate | advisory | parent | route | where the fix lands |
|---|---|---|---|---|
| `paste` (first parent) | RUSTSEC-2024-0436 | `frankenstein` 0.52.1 | a fork at the release takes `pastey` 0.2.3, already in the lock, at its ten `paste!` calls | RexRenatus/frankenstein, then `[patch.crates-io]` |
| `paste` (second parent) | RUSTSEC-2024-0436 | `uniffi_bindgen` 0.29.1 | a fork at the release drops the declared and unused dependency, as upstream did after it | RexRenatus/uniffi-rs, then `[patch.crates-io]` for its seven crates |
| `basic-toml` | RUSTSEC-2026-0374 | `rinja_derive` 0.3.5 | a fork at the release parses the config with `toml` 0.5, already in the lock, at its one call | RexRenatus/askama, then `[patch.crates-io]` for `rinja`, `rinja_derive` and `rinja_parser` |

`paste` is cured only when both of its parents' rows land.

### Criteria

A25 is new, in `scripts/tests/test_engine_pin.py`. It reads the lockfile's packages and refuses each
crate this SPEC's amendments retire, at any version and from any source; a planted lock holding each
crate reads one finding apiece and stays red after the cure, and a clean planted lock reads none.
A1's exact `allow-git` list grows by the three forks, each with a planted `deny.toml` that leaves it
out. A3 is unchanged: six exceptions remain, each one encountered, and each of the eight allowed
sources is in the graph.

### Red first

Each notice is red first: `cargo deny --locked check advisories`, with its ignore entry removed,
fails naming it before its route lands, and passes after, with the other six entries applied. A25 is
red over the lockfile before the first fork lands, and green once the last one does. The record is
`docs/red-first/SPEC-055.md`, which takes the reds in the commit that removes the two entries.

### Manifest

| file | change |
|---|---|
| `deny.toml` | changed: the two ignore entries and their comment leave; `allow-git` names the three forks, and the `[sources]` comment counts them |
| `Cargo.toml` | changed: `[patch.crates-io]` takes the three forks by `rev`, each under an ADR-657 comment |
| `Cargo.lock` | changed by cargo: `paste` and `basic-toml` leave; the patched crates come from their forks |
| `scripts/tests/test_engine_pin.py` | changed: A25 added; A1's exact `allow-git` list grows by three, each with a planted `deny.toml` |
| `docs/specs/SPEC-055-the-engine-moves-to-26-09-3-on-a-patched-fork.md` | changed: this section, and A25's row and fence line |
| `docs/red-first/SPEC-055.md` | changed: A25's red and green, and each notice's |
| `docs/decisions/ADR-657-three-parent-crates-come-from-forks-until-their-releases-drop-paste-and-basic-toml.md` | added |
| `changelog.d/fix-deps-unmaintained-paste-basic-toml.md` | added |

### What this does not do

- It keeps the six engine notices, RUSTSEC-2025-0075, 0080, 0081, 0094, 0098 and 0134, until the
  engine's pin moves off them (#256, #248 to #253).
- It writes no `unmaintained` key; cargo-deny's default scope applies until those six leave (#256).
- It keeps the bot surface and its daemon wiring, so `frankenstein` keeps its fork until they are
  retired (#1046).
- It does not upgrade `uniffi`; ADR-345 pins it (#1001).

### Risks

- `toml` 0.5 is an old line; it is already in the lock through the bindings generator, and the fork
  entry goes when a `rinja` or askama release the generator takes drops `basic-toml`.
- `uniffi_internal_macros` moves from 0.29.5 to 0.29.1, the release's own pairing; the XCFramework
  workflow's bindings, modulemap and consumer checks measure the generated code on every run.
- A fork branch that stops being fetchable breaks a fresh build; `allow-git` names each fork, and A3
  refuses an allowed source that no crate comes from.
- Open pull requests that change `Cargo.lock` conflict with this one textually; the later lander
  regenerates the lock with cargo, never by a text merge.

### Links

#1001, #326, https://github.com/mozilla/uniffi-rs/pull/2485,
https://github.com/askama-rs/askama/pull/771 and https://github.com/askama-rs/askama/issues/794.
The upstream pull requests this fix owes are linked here when opened.
Opened: https://github.com/askama-rs/askama/pull/796 (`basic-toml`) and https://github.com/ayrat555/frankenstein/pull/342 (`paste`).

## 10. Amendment: the engine's six unmaintained notices leave deny.toml (#256)

### Problem

After §9, `deny.toml` carried six `[advisories]` ignore entries, each an unmaintained notice with no
patched release: RUSTSEC-2025-0075, 0080, 0081, 0094 and 0098 (the five `unic` crates) and
RUSTSEC-2025-0134 (`rustls-pemfile`), and it set no `unmaintained` key. An exception is not a fix,
and a narrowed `unmaintained` scope is one too. The lockfile census (tomllib over `Cargo.lock`, 654
packages): the engine alone names `unic-ucd-category` and `rustls-pemfile`, and the other four
`unic` crates and `matches` are named only under `unic-ucd-category`, so these seven are all the
lock loses when the engine drops its two. A read of every advisory file of every locked crate finds
these six notices and no other that a locked version meets. Issues #248 to #253 and #256 track them.

### Requirements

R15. `deny.toml` holds no `[advisories]` ignore entry in either form and sets `unmaintained = "all"`,
    the widest value and cargo-deny's default, so that no later default narrows it (A24), and A3
    refuses any advisory exception.
R16. The five `unic` crates, `rustls-pemfile` and `matches` leave the lockfile through the engine's
    pin (A25). No other package leaves it or enters it: the engine packages' source moves, and the
    engine's own dependency list loses the two crates; the lock's diff is read at review.
R17. The engine's pin moves to the head of the fork's branch `deckstreak-blockbuf` once
    RexRenatus/anki#1 is merged there (ADR-058's note). Every engine fact this SPEC and
    `test_engine_pin.py` name is re-read at the new pin (A1 to A25); every file that names the pin
    moves with it (the parity oracle's driver, its stand-in harness, the golden tests' planted
    golden and the runbook's engine input); and the engine's goldens, their census and the schema
    snapshot are written again at the new commit by the runbook's sandbox run
    (`tools/parity-oracle/anki/RUNBOOK.md`), never by a builder or by CI.
R18. This fix owes no upstream pull request: the engine's two fixes stay on DeckStreak's maintained
    fork, and nothing goes to the engine's upstream repository, until the engine leaves the build.

| crate | advisory | parent | route | where the fix lands |
|---|---|---|---|---|
| `unic-char-range`, `unic-common`, `unic-char-property`, `unic-ucd-category`, `unic-ucd-version` | RUSTSEC-2025-0075, 0080, 0081, 0094, 0098 | the engine (`rslib`: `media/files.rs` and `typeanswer.rs`) | the two call sites move to frozen Unicode 10 predicates generated from `unic-ucd-category` and proved equal to it on every code point, so names and answers are unchanged; `matches` leaves with `unic-ucd-category` | RexRenatus/anki#1, then this repository's `[patch]` rev |
| `rustls-pemfile` | RUSTSEC-2025-0134 | the engine (`rslib/src/backend/mod.rs`) | the PEM item count moves to `rustls-pki-types`' PEM reader, the advisory's own successor, already in the lock; eight goldens equal | RexRenatus/anki#1, then this repository's `[patch]` rev |

### Criteria

A24 is new, in `scripts/tests/test_engine_pin.py`. It reads `deny.toml` and refuses each
`[advisories]` ignore entry in either form and an `unmaintained` key that is absent or not `"all"`;
a planted file holding each defect reads one finding apiece and stays red after the cure, and a
clean plant reads none. A3 now refuses any advisory exception and any exception cargo deny applies,
and keeps every allowed git source live; its row in §3 keeps the words it was decided with. A25's
list of retired crates grows by the six, by insertion; its assertions are unchanged.

### Red first

With the six entries removed and `unmaintained = "all"` written, over the lock the base holds,
`cargo deny --locked check advisories` fails naming exactly the six, and once the pin moves it
passes with no ignore entry. A24 is red over the base's `deny.toml` and green once it is edited; A3's
new form is red over the base's six entries; A25 is red over the base's lock for the six crates and
green at the pin. The record is `docs/red-first/SPEC-055.md`, which takes the reds in the commit that
removes the six entries and the greens in a later commit.

### Manifest

| file | change |
|---|---|
| `deny.toml` | changed: the six ignore entries and their comment leave; `unmaintained = "all"` |
| `Cargo.toml` | changed: the engine's `[patch]` rev moves, with its comment |
| `Cargo.lock` | changed by cargo: the five `unic` crates, `rustls-pemfile` and `matches` leave; the engine packages come from the new commit |
| `scripts/tests/test_engine_pin.py` | changed: A24 added; A25's retired list grows by six, by insertion; A3 refuses any exception |
| `tools/parity-oracle/anki/generate.py` | changed: the engine commit it pins, and the harness pin when the runbook's first step moves it |
| `tools/parity-oracle/anki/fixtures/stand_in_harness.py`, `tools/parity-oracle/test_goldens.py`, `tools/parity-oracle/anki/RUNBOOK.md` | changed: the engine commit they name |
| `tools/parity-oracle/goldens/anki.*.json`, `tools/parity-oracle/anki/stock-lines.sha256`, `tools/parity-oracle/anki/schema-snapshot.sql` | changed: written again at the new engine commit by the runbook's run |
| `docs/specs/SPEC-055-the-engine-moves-to-26-09-3-on-a-patched-fork.md` | changed: this section, and A24's row and fence line |
| `docs/red-first/SPEC-055.md` | changed: A24's red and green, and each notice's |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | changed: a note on the new pin |
| `changelog.d/fix-deps-unmaintained-unic-rustls-pemfile.md` | added |

### What this does not do

- It leaves cargo-deny's `unsound` scope at its default; widening it is its own decision
  (#1262).
- It does not move the engine to current Unicode data; the frozen table keeps media names and typed
  answers equal to released clients until an engine release carries a change (#256).

### Risks

- The frozen Unicode 10 table can drift from a later upstream move to current data; the fork entry
  goes when an engine release drops `unic`, and its tests read the table against the release then.
- An unmaintained notice published before the merge reds the audit with no entry to hold it; it is
  then its own fix, never an ignore entry (R15).
- The release's sync server audit reads the fork's lock with this `deny.toml` and no ignore entry; an
  advisory there reds the release before any build, and is fixed the same way.
- Open pull requests that change `Cargo.lock` conflict with this one textually; the later lander
  regenerates the lock with cargo, never by a text merge.

### Links

#256, #248 to #253, https://github.com/ankitects/anki/issues/5882 and
https://github.com/RexRenatus/anki/pull/1. This fix owes no upstream pull request (R18).
