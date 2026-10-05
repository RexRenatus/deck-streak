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
