# SPEC-630: a recorded study day measures the bench membership, and every pull request checks the benches reach it

- **Context(s):** `repo` (`scripts/perf-census.py`, `scripts/tests`, `scripts/mutation-python.json`;
  in the second part `perf/`, `crates/ffi/examples/`, `.github/workflows/ci.yml`, `scripts/check.sh`
  and `scripts/perf-plan.py`), `docs`.
- **Decided by:** ADR-652 (this SPEC's own: callgrind, the membership, the reach, the share, the
  release-run derive and the check), SPEC-618 and ADR-640 (the benches, the benchmark list and the
  bench plan).
- **Status:** in progress on its pull request: the first part delivers the documents and
  `scripts/perf-census.py`, its `derive` parse and write half and its `check`, with their tests;
  the second part adds the rest, as section 7 says.

## 1. The problem, measured

At dev `6c3af8f`:

- The benches are chosen by hand: `perf/benchmarks.txt` names 11 benches, and no file records
  which functions a learner's day spends its instructions in:
  `git ls-tree -r --name-only 6c3af8f -- perf | grep -c -E 'hot-set|bench-reach'` reads 0.
- A bench joins only by review, so nothing tells a pull request that deletes a bench whether a hot
  function is still measured, and the wave 2 benches wait on a census that names their functions.

## 2. Requirements

R1. `perf/workload.json` records the study-day scenario as numbers only: seed, cards, notes, decks,
review-log length, presses, faces and the replay's deck set, shaped to measured usage counts; it
holds no collection and nothing personal. A change to it is a reviewed pull request, and a change
that drops a member whose code still exists is disclosed.

R2. `crates/ffi/examples/study_day.rs` runs one study day in the `bench` profile: it builds the
synthetic collection unmeasured, then calls `measured()`, an `#[inline(never)]` function that
opens it through `Engine::new`, resolves the day's queue, renders the question and answer face of
200 cards, answers each one and runs the FSRS-7 replay over the deck set. An inlined `measured()`
would leave the toggle no symbol to match, and every derive would read VOID.

R3. The profiler is Valgrind's callgrind, run as `valgrind --tool=callgrind --collect-atstart=no
--toggle-collect='*study_day::measured*'`, so only `measured()` is counted (Valgrind manual
6.2.2 and 6.3.3: the toggle already starts with collection off; the flag states it). The Valgrind
version is recorded beside the membership, and a derive refuses a listing another version made.

R4. The membership M at share S is the list `callgrind_annotate --inclusive=no --threshold=<S>
--auto=no` prints: functions by exclusive cost, printed until the sum of their shares is at least
S (manual 6.6), with no formula of our own. The denominator is every instruction in the measured
region: our crates, the Anki engine, SQLite, the standard library and libc.

R5. Each name is normalized: a Rust legacy hash suffix (`::h` and 16 hex digits) is removed, the
path and the file are kept, and the entries one name and file share after that are one member,
their counts summed. A file under the checkout's root is written relative to it.

R6. `derive` writes `perf/hot-set.json`, schema `deckstreak.perf-hot-set.v1`, with exactly the
keys `schema`, `share`, `workload` (`path`, `sha256`), `derived_at`, `release`, `toolchain`,
`valgrind`, `total_ir` and `members` (each `fn`, `file`, `self_ir`, `share`, by `self_ir`
descending), and `perf/bench-reach.json`, schema `deckstreak.perf-bench-reach.v1`, with exactly
`schema`, `derived_at`, `benches` (each bench id to its `total_ir` inside its toggle and its
`functions`) and `files`. A share is a percentage. The facts a listing cannot give (the workload
file's path and sha256, `derived_at`, `release`, the toolchain and the Valgrind version) come from
its arguments.

R7. Each Rust bench's measured body is the `#[inline(never)]` function `<bench>_measured` its
Divan closure calls. Its reach run collects only inside it (`--collect-atstart=no
--toggle-collect='*<bench>_measured*'`, `--sample-count 1 --sample-size 1`), so setup, Divan's
input closures and process start are no reach; the normalized functions with an instruction count
above 0 there are its reach. A second run of each bench binary with no toggle lists the source
files the whole binary reaches, setup included: `files`, the files under the root, which only the
bench plan's applicability reads, so that a change to setup code still runs the bench.

R8. S is 90% of exclusive instructions, fixed after a measurement report: determinism (two derives
on one commit give one M, else VOID and the cause is found first), the curve (|M| at 80, 90, 95
and 99%, each split into our code and third-party code), reach (how many of each M the seven Rust
benches reach) and churn (M at two dev tips at least 50 commits apart). 90% stands while |M(90%)|
is 60 functions or fewer; above 60 the curve goes to the maintainers, because a lower share is a
weaker gate. Raising S later is a reviewed change; lowering it is the maintainers' decision.

R9. Each release: a `perf-census` job of `ci.yml` applies only on the release run (`base-is-dev`'s
`release` output) and is not applicable on every other run. It checks out the release pull
request's head (`github.event.pull_request.head.sha`), derives there and records that head as
`derived_at`, never the merge ref, and uploads `perf-census-<head8>` (both files and the listing).
It is not in `ci`'s needs, so it informs and never holds a release.

R10. The refresh: after a release is tagged, a pull request into dev commits, byte-identical, the
two files of the artifact whose head is the new tag's dev parent (`<tag>^2`), and adds any bench a
new member needs. It merges before the next release is tagged.

R11. `scripts/perf-census.py check --root <dir>` runs in the `hygiene` job's python stage with no
build and no Valgrind. It reads the two files under `<dir>/perf/`, the bench files of the tree at
`<dir>` and the git history there, and judges by arm, each line `perf-census: <arm>: ...`:
- `shape`: a closed schema for both files (an unknown or a missing key, a value outside R6's
  types, a schema name other than R6's, or two different `derived_at`); the other arms run only
  over a shape it admits;
- `unreached`: a member outside the union of the reach of the benches the tree still defines: the
  `#[divan::bench]` functions of `crates/*/benches/*.rs` and the `bench(` calls of
  `web/app/bench/**/*.bench.ts`, each named `<file stem>::<bench>` as `perf/benchmarks.txt` names
  them, found by a static scan;
- `stale`: `derived_at` is not an ancestor of HEAD, or it is the dev parent (`<tag>^2`) of neither
  the newest SemVer release tag (`v<major>.<minor>.<patch>`, ordered by number) nor the one before
  it; while one release tag exists only its dev parent qualifies; a tag it reads with no second
  parent reads VOID, because a release always merges with a merge commit, and so does a history
  with no release tag;
- `void`: an empty M, a `total_ir` of 0, or a bench whose `total_ir` inside its toggle is 0;
- `edge`: informational, each member whose cumulative share ends within 0.5 percentage points of S
  prints its position, so a flap at the next release is foreseen.

R12. The ramp: `check` is report-only by default, printing every finding and exiting 0, until the
first refresh pull request after this delivery. `--enforce` makes `shape`, `unreached`, `stale`
and `void` exit 1, a VOID included, as hygiene will from then; `edge` never refuses.

R13. `derive`'s parse half refuses, exits 1 and writes nothing, on a listing it cannot read as
`callgrind_annotate`'s (its creator, events, thresholds, totals, function header and function
lines), a creator other than `callgrind-<valgrind version>`, events other than `Ir`, a hot-set
listing printed at a threshold other than S, a reach or untoggled listing printed at a threshold
other than 100 (every function), a hot-set list that goes on after its printed share reached S or
that sums short of S, and a hot-set listing that counted 0 instructions.

R14. The script is standard library only. `derive`'s build and profile half (the bench-profile
build and the callgrind runs) is its own function, added with the release-run job; no test of the
first part calls it.

## 3. Acceptance criteria of SPEC-630's first part

| id | criterion | decided by |
|---|---|---|
| A1 | `derive` writes both documents, every key, normalized names and merged entries, files under the root relative to it, and the arguments' facts; it refuses another creator, other events or a line that is no function line; and it refuses a listing that does not stop at the share, or a reach listing not printed whole, writing nothing (R4 to R6, R13) | `test_perf_census.py` `test_derive_writes_both_documents_with_normalized_names`, `test_derive_refuses_a_listing_that_does_not_stop_at_the_share` |
| A2 | `shape` refuses an unknown or missing key, a value outside the schema and two different `derived_at` (R11) | `test_perf_census.py` `test_shape_refuses_an_unknown_or_missing_key_and_a_value_outside_the_schema` |
| A3 | `unreached` refuses a planted member no bench reaches, and a member whose only bench was deleted (R11) | `test_perf_census.py` `test_unreached_refuses_a_member_no_bench_reaches`, `test_unreached_refuses_a_member_whose_bench_was_deleted` |
| A4 | `stale` refuses a `derived_at` two releases behind and a merge-ref `derived_at`, judges one tag alone, and reads VOID on a tag with no second parent (R11) | `test_perf_census.py` `test_stale_refuses_a_derived_at_two_releases_behind`, `test_stale_refuses_a_merge_ref_derived_at`, `test_stale_reads_void_on_a_tag_with_no_second_parent_and_judges_one_tag_alone` |
| A5 | `void` refuses an empty hot set and a bench with 0 instructions inside its toggle, and `edge` prints a member near the cut and never refuses (R11, R12) | `test_perf_census.py` `test_void_refuses_an_empty_hot_set`, `test_void_refuses_a_bench_with_no_instructions_inside_its_toggle`, `test_edge_prints_a_member_near_the_cut_and_never_refuses` |
| A6 | the controls: `stale` admits a `derived_at` one release behind, and report-only prints and exits 0 over a tree every arm refuses (R11, R12) | `test_perf_census.py` `test_stale_admits_a_derived_at_one_release_behind`, `test_report_only_prints_and_exits_0` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_derive_writes_both_documents_with_normalized_names
A1: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_derive_refuses_a_listing_that_does_not_stop_at_the_share
A2: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_shape_refuses_an_unknown_or_missing_key_and_a_value_outside_the_schema
A3: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_unreached_refuses_a_member_no_bench_reaches
A3: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_unreached_refuses_a_member_whose_bench_was_deleted
A4: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_stale_refuses_a_derived_at_two_releases_behind
A4: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_stale_refuses_a_merge_ref_derived_at
A4: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_stale_reads_void_on_a_tag_with_no_second_parent_and_judges_one_tag_alone
A5: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_void_refuses_an_empty_hot_set
A5: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_void_refuses_a_bench_with_no_instructions_inside_its_toggle
A5: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_edge_prints_a_member_near_the_cut_and_never_refuses
A6: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_stale_admits_a_derived_at_one_release_behind
A6: python3 -m unittest discover -s scripts/tests -p test_perf_census.py -k test_report_only_prints_and_exits_0
```

Each test drives the script as a child process over listings from
`scripts/tests/fixtures/perf-census/` and a tree planted in a temporary directory. The `stale`
reads take git through the root the caller passes: each `check` test builds a throwaway repository
there (`git init`, empty commits, merge commits whose second parent is a dev commit, annotated
SemVer tags and one tag that is no release), so no test reads this repository's history, makes a
network call, runs Valgrind or builds. A1 to A5 are red at the red commit by an assertion against
a stub that keeps every argument and lacks each behaviour: its `derive` writes both documents with
no member, no bench and no file and accepts every listing, and its `check` prints only its summary
line and exits 0. A6 is not red: its two tests are the controls beside the refusal arms, a
`derived_at` one release behind and the report-only exit, which the stub reads green at both
pushes.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-630-a-recorded-study-day-measures-the-bench-membership-and-every-pull-request-checks-the-benches-reach-it.md` | docs | added |
| `docs/decisions/ADR-652-the-bench-membership-is-what-callgrind-annotate-prints-at-ninety-percent-of-a-study-days-instructions.md` | docs | added |
| `docs/schematics/perf-census.md` | docs | added |
| `changelog.d/perf-census-630.md` | docs | added |
| `docs/red-first/SPEC-630.md` | docs | added |
| `scripts/perf-census.py` | scripts | added: `derive`, its parse and write half, and `check` |
| `scripts/tests/test_perf_census.py` | tests | added |
| `scripts/tests/fixtures/perf-census/` | tests | added: callgrind_annotate listings, the two committed files and the bench files of a planted tree |
| `scripts/mutation-python.json` | mutation | changed: one entry, the script with its test module |

## 5. What this does NOT cover

- The web census over Node's CPU profile, by self-time share in at least 4 of 5 runs, with `-web`
  files and the same arms: it joins with the wave 2 benches (#1149).
- The wave 2 benches themselves (opening and queueing, the syncs, the wasm adapter's next card):
  each joins only once this census puts its functions in its membership (#1149).

## 6. Risks

- **A listing format changes.** A Valgrind upgrade could print another layout; the parse refuses
  what it cannot read and the recorded version refuses another creator, so a changed layout reads
  as a refusal, never as an empty membership.
- **A name differs between the study day and a bench binary.** Membership and reach are matched by
  normalized name; a monomorphized or renamed symbol that differs reads `unreached`, which a
  reviewer then reads by name.
- **The release history is read from a shallow clone.** `stale` needs the tags and their parents;
  hygiene's checkout fetches the full history (`fetch-depth: 0`, `.github/workflows/ci.yml:316`),
  and a history with no release tag reads VOID, never OK.

## 7. What each part of this pull request delivers

The first part delivers R5, R6 and R11 to R13 in `scripts/perf-census.py`, with criteria A1 to
A6. The second part, later in this same pull request, adds `perf/workload.json`, the study-day
example, the `perf-census` release-run job, the hygiene stage line in `scripts/check.sh`, the bench
plan's `files` read and the first `perf/hot-set.json` and `perf/bench-reach.json` with R8's
measurement report, each with its own manifest rows and criteria (R1 to R4, R7 to R10, R14).

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani (ADR-652 D7): the first part adds documents, one
standard-library script and its tests, with no engine path, protocol or state machine.
