# SPEC-493: the release run carries each Python mutant outcome a recorded release run judged on the same read set, and runs only the rest

Path:
`docs/specs/SPEC-493-the-release-run-carries-each-python-mutant-outcome-a-recorded-release-run-judged-on-the-same-read-set.md`.

- **Issue:** refers to #822, the release pull request from `dev` into `main`; closes nothing.
- **Context(s):** the mutation scripts (`scripts/`), their tests and fixtures, `ci.yml`'s
  `mutation-plan` and `mutation-verdict` jobs, one new Lean entry under `formal/lean/`. No crate
  changes and no web changes.
- **Decided by:** ADR-511 (a release run carries an outcome only from one base of this repository,
  selected without reading any outcome, and only for a file whose whole read set, absent paths and
  directory listings included, is byte-identical at the base and at the head). SPEC-039 R3 and
  ADR-057 gain one insert-only amendment each. SPEC-422 and the release-legs ruling are not
  amended by this delivery (section 5).
- **Schematic:** `docs/schematics/mutation-release-carry.md` (data flow: listing, carry, deal, judge).
- **Mutation band:** `S49300-S49399`, rows in `scripts/mutation-rows.d/S49300-S49399.json`.
- **Red-first record:** `docs/red-first/SPEC-493.md`.
- **Status:** delivered by the pull request that adds this file, with its tests, its fixtures, its
  rows, its Lean entry and its red-first record.

## 1. The problem, measured

Every figure below was read at `dev` `75d555c4`; each line names its command.

- The release run lists every mutant `dev` gained since the last release and runs each one.
  `scope_of` (`scripts/mutation-verdict.py:203-212`) names the release case, `release_run`
  (`:230-240`) names the release run, and the listing steps (`ci.yml:424-438`) list the diff's
  and the whole population's mutants; `shards` (`ci.yml:441`) deals every listed Python mutant into
  legs (`git -C <clone> show 75d555c4:<path> | sed -n '<range>p'`).
- So a release pull request whose run is repeated after a small change re-runs every listed mutant,
  though a recorded run already judged most of them on the same bytes.
- The obvious shortcut, carrying a mutant's outcome when its own file is unchanged, is unsound. A
  test can read data its closure never imports: `scripts/tests/test_threat_model.py:212` runs
  `threat_model.judge(ROOT)` on the real tree, which reads every `docs/schematics/*.md`
  (`scripts/threat_model.py:18`, `:86`), so a new schematic can change a verdict while every
  Python file is unchanged. A test can probe a path that is absent (a module that would shadow an
  import, a file a directory walk would meet), so agreement on the files a run opened is not
  agreement on what it probed. A proof of the narrowed obligation (section 2's R6 and R7) shows
  both, and shows that a base chosen by its outcomes passes more often than one draw.
- The Python population is 16 files (`scripts/mutation-python.json`, 16 keys). A static census of
  each file's closure (the file, its mapped test modules and their in-repo imports, read by `ast`
  at `75d555c4`) finds, in 14 of the 16, at least one member with a construct no static read set
  can bound: a process other than the interpreter, a directory walk, a read of the working
  directory, the repository root passed to a call, or a dynamic import. The census reads 0 fenced
  paths (`python3 -I census.py <clone> 75d555c4 <listing> scripts/mutation-python.json <out>`).
- The runner `scripts/mutation_python.py` imports `scripts/mutation_rows.py` and uses two of its
  names, `sha256` and `tracked_changes` (`scripts/mutation_python.py:795`, `:798`, `:1442`).
- Rust tests read the tree outside their crates: `crates/notifications/tests/request_allow_list.rs:564-589`
  reads every Rust source, manifest, script and workflow of the repository, and
  `crates/progression/tests/xp_census.rs:396-404` walks every directory. 203 `.rs` files in 26 of
  31 packages hold a token that can reach outside the crate (`git grep -l -E
  'CARGO_MANIFEST_DIR|"\.\./|\.\./\.\.|\.parent\(\)|read_dir|WalkDir|walkdir|current_dir|include_str!|include_bytes!|Command::new'
  75d555c4 -- . <fences>`, then `^crates/.*\.rs$`). No Rust read set is sound without a
  per-package census, so this delivery carries no Rust outcome.
- `mutation-plan` runs under the workflow's default token, `contents: read` (`ci.yml:19-20`), and
  holds no job-level `permissions`; downloading another run's artifact needs a token with
  `actions: read` and that run's id (the download action's own documentation).
- `scripts/mutation-verdict.py`'s `judge_python`, `shards`, `whole_reports` and `partition`, and
  `scripts/mutation_python.py`'s `planned_leg`, `child_trace` and `scoped_sets`, are covered spans
  of three formal entries (`git grep -n '@phx covers' 75d555c4 -- . <fences>`): this delivery
  edits none of them.

## 2. Requirements

R1. **Admission.** The carry applies only to the release run (`release_run`) of a release pull
    request `CARRY_PULL_REQUESTS` names. `CARRY_PULL_REQUESTS = frozenset({822})` in
    `scripts/mutation_carry.py` has one `#:` line directly above it. On every other run `carry`
    writes a carry that holds nothing and says why by name.
R2. **The base, found under a read-only token.** `mutation-plan` holds job-level `permissions`
    of exactly `contents: read` and `actions: read`; the workflow's default stays `contents: read`.
    One step lists this repository's `ci` workflow runs for `pull_request` events from `dev`, and
    `mutation_carry.py base` selects the base from that listing by run id alone: the newest run,
    older than this run, whose workflow path is `.github/workflows/ci.yml`, event `pull_request`,
    head branch `dev`, head repository this repository, with a pull request into `main`, status
    `completed`, and conclusion neither `cancelled` nor `skipped`. Its head must be an ancestor of
    this run's head, read from the forge's compare. Exactly one candidate is judged: when it fails
    any test there is no base, never an older one. The selector reads no mutant outcome. A fork's
    run never qualifies.
R3. **Fetching.** The base's `mutation-plan` artifact, and its `mutation-outcomes` artifact when the
    base lists one (else its `mutation-python-shard-*` reports), are downloaded by the pinned
    download action with the base's run id and the workflow token, into a directory the plan's own
    upload does not hold. Each download step is bounded and continues on error, so a missing or
    expired artifact is no base and never fails the plan.
R4. **Tree identity.** A base qualifies only when its plan is a release plan and its `base` (the
    merge ref's first parent) equals this plan's. A path is **changed** when its section of the
    base's `git.diff` and its section of this run's differ byte for byte, a path in neither diff
    being equal; every binary section is changed. A directory's **listing** is changed when a
    changed path below it is present on exactly one side.
R5. **The key.** For each Python class the key is equal only when (a) the text of the jobs
    `mutation-python` and `mutation-python-release`, comment and blank lines set aside, is equal at
    the base and here; (b) `scripts/mutation_python.py` is byte-identical; and (c) the runner digest
    of `scripts/mutation_rows.py` is equal: the sha256 over the syntax trees of every top-level
    statement that is not a `def` or `class`, and of each function or class `mutation_python.py`
    names as `mutation_rows.<name>`, closed over the module-level names their bodies reference. A
    base file is rebuilt as the merge-base blob with the base's section applied; a rebuilt file
    whose git blob id does not begin with its section's `index` id is no base. An unequal key
    carries nothing for its class.
R6. **The read set (O3').** For each Python population file `f`, `mutation_carry.read_set` computes
    `S(f)` at the head over `f`'s closure: `f`, its mapped test modules, and every in-repo module a
    member imports, transitively, or loads by a constant path. `S(f)` holds every member; for each
    top-level name a member imports and each directory the child's `sys.path` holds, the probes
    `<dir>/<name>.py`, `<dir>/<name>/` and `<dir>/<name>.<suffix>`; every path a constant path
    expression in the closure names, a directory with its whole subtree and every listing in it,
    with the listing of each ancestor up to the root, and each chain anchored on `__file__`
    re-anchored on every ancestor of its anchor; and `f`'s entry of `scripts/mutation-python.json`.
    `S(f)` is the **whole tree** when any member holds a construct outside the allowlist of R7.
    The census reads every file it reaches, parked files included, and names paths by rule or glob.
R7. **The census allowlist.** A member is bounded when its file-system, process, environment and
    import constructs are only: imports; loader calls (`spec_from_file_location`, `run_path`) and
    interpreter subprocesses whose script is a constant in-repo path; `tempfile` values and file
    operations on a path derived from one in the same function; and reads, stats and listings of a
    constant path expression. Anything else makes `S(f)` the whole tree: another process, an
    `os.system`, `os.popen`, `exec*` or `spawn*` call; a read, stat or listing of a path that is
    neither constant nor temporary; the repository root, the working directory or `Path()` passed
    to any call but a path join or a loader; `exec`, `eval`, `compile`, `__import__` or
    `import_module` of a non-constant; an import of `ctypes`, `mmap`, `sqlite3`, `posix` or `_os`;
    a `sys.path` entry that is not constant; an `os.environ` or `getenv` read; or a member that does
    not parse.
R8. **Carry per file.** All of `f`'s listed mutants carry, or none does. They carry only when: the
    run is admitted (R1); there is a base (R2 to R4); the key of `f`'s class is equal (R5); `f`'s
    map entry is equal at the base and here and the map's census at the head finds no problem;
    no path of `S(f)` is changed and no listing of it is (a whole-tree `S(f)` needs no changed path
    at all); and every mutant of `f` listed here is judged once at the base with the outcome
    `killed`, `survived`, `uncovered` or `unviable`, in a leg the verdict's own Python reader reads
    as whole against the base's plan, or carried by the base's manifest, each name occurring once on
    each side. Otherwise the file's reason is printed by name and every one of its mutants runs.
R9. **The legs' view.** `carry` writes `carry.json` (schema `deckstreak.mutation-carry.v1`) first
    and then, in `plan.json`, moves each carried file's `code` lines to `carried_code`; nothing else
    in the plan changes. The runner lists `python-rerun.json` from that plan, and `shards` deals it.
    The listing steps, `shards`, `planned_leg` and the runner itself are unchanged.
R10. **The union rule (O1).** The verdict's Python reader reads `carry.json` and `python-listed.json`
    from the plan's artifact. For each class, the names `python-listed.json` lists equal, as
    multisets, the names the whole re-run legs examined plus the carried names, and no name is in
    both; any other reading is VOID by name with its three counts. A missing `carry.json` reads as
    nothing carried.
R11. **Judged at the head (O5).** A carried outcome is judged by the rules a re-run one is: a
    survivor fails unless exactly one record at the head excuses it, an uncovered mutant fails, and
    a kill refutes a record at the head. The equivalence records, the rows retirement check and
    `legs` are evaluated at the head and never carried. The class prints `carried N from run R`
    apart from its examined count, and a class that applies with nothing re-run and nothing carried
    is VOID.
R12. **The manifest.** The verdict job writes and uploads `mutation-outcomes` (schema
    `deckstreak.mutation-outcomes.v1`) under `if: always()`: this run's tree identity (the plan's
    base and head and `git.diff`'s sha256), each Python class's key, and one entry per Python
    mutant judged in a whole leg or carried (name, file, class, outcome, killers, the run that
    examined it). No partial or VOID leg's mutant, no `timeout` and no `void` outcome enters it.
R13. **Never carried.** Rust, rows and web outcomes (they run as before); a `timeout` or `void`
    outcome; any outcome when the run is not admitted.
R14. **Fail-safe.** No base, an expired or unreadable artifact, a moved `main`, an unreachable
    head, an unequal key, a changed path or listing of a read set, a closure member that does not
    parse, a name held twice, or an error inside `carry` each carry nothing for the part they
    touch, by name, and that part runs whole. No bound, price, timeout, ceiling or per-mutant
    budget changes.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R9, R10 (O1): the union rule passes the exact partition and is VOID by name with three counts on a missing, an extra and a doubly held name; a missing `carry.json` reads as nothing carried; the legs' view moves only carried files' lines | `test_mutation_carry.py` `UnionRule` |
| A2 | R4, R6, R7, R8 (O2, O3'): a file carries only when no path of its read set, present or absent, and no listing of it changed; planted controls: a root reader like `test_threat_model.py` is whole-tree, an added module that shadows an import is changed, an absent probe added at the head is changed, a binary section is changed, and the own-file-only rule is refused | `test_mutation_carry_census.py` `ReadSet`, `PlantedControls` |
| A3 | R1, R2, R3 (O4): the selector takes the newest qualifying run by id; a fork's run, a cancelled run, another workflow, another branch, a run whose head is not an ancestor and a run with no plan artifact each give no base; no outcome is read; `mutation-plan`'s permissions and download steps are as R2 and R3 state | `test_mutation_carry.py` `BaseSelection`, `Workflow` |
| A4 | R5, R13, R14: a changed leg job, runner file or referenced `mutation_rows` name, or a changed map entry, carries nothing; an edit to an unreferenced `mutation_rows` function keeps the key; every fail-safe path carries nothing by name | `test_mutation_carry.py` `Key`, `FailSafe` |
| A5 | R11, R12 (O5): a carried survivor is excused only by a record at the head, a carried kill refutes one; the manifest holds whole and carried entries only | `test_mutation_carry.py` `JudgedAtHead`, `Manifest` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k UnionRule
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_carry_census.py -k ReadSet
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_carry_census.py -k PlantedControls
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k BaseSelection
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Workflow
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Key
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k FailSafe
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k JudgedAtHead
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_carry.py -k Manifest
```

Each test prints the population it examined as `examined N`, and N is above 0.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_carry.py` | 1 | new: the selector, tree identity, key, census, carry and union rule, and the manifest's entries; standard library only |
| `scripts/mutation-inputs.json` | 1 | new: the read-set map, each class's rule (`closure` or `whole-tree`), the leg jobs of its key and the runner files, by rule or glob |
| `scripts/mutation-verdict.py` | 1 | the `carry` and `manifest` verbs; `python_reports` reads the carry and holds the union rule; no covered span changes |
| `scripts/mutation-python.json` | 1 | one entry: `scripts/mutation_carry.py` and its two test modules |
| `.github/workflows/ci.yml` | 1 | `mutation-plan`: job-level permissions, the base and download steps, `carry`, the re-run listing, and `shards` reading it; `mutation-verdict`: the manifest step and its upload |
| `scripts/tests/test_mutation_carry.py` | 1 | new: A1, A3, A4, A5 |
| `scripts/tests/test_mutation_carry_census.py` | 1 | new: A2 |
| `scripts/tests/fixtures/mutation-carry/**` | 1 | new: fixture trees, diffs, run listings and leg reports |
| `scripts/mutation-rows.d/S49300-S49399.json` | 1 | new: the band's rows (section 9) |
| `formal/lean/Formal/ReleaseCarry.lean` | 1 | new: the entry (section 8) |
| `formal/lean/Formal/ReleaseCarryVectors.lean` | 1 | new: its vectors writer |
| `formal/lean/Formal/Vectors.lean` | 1 | one import and one arm |
| `formal/lean/Formal.lean` | 1 | one import |
| `formal/vectors/release-carry.jsonl` | 1 | new: written by the formal checker, never by hand |
| `docs/specs/SPEC-493-the-release-run-carries-each-python-mutant-outcome-a-recorded-release-run-judged-on-the-same-read-set.md` | 1 | new: this file |
| `docs/decisions/ADR-511-a-release-run-carries-an-outcome-only-from-one-own-base-on-a-byte-identical-read-set.md` | 1 | new |
| `docs/schematics/mutation-release-carry.md` | 1 | new |
| `docs/red-first/SPEC-493.md` | 1 | new |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | 1 | one insert-only amendment section |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | 1 | one insert-only note |
| `changelog.d/release-carry-822.md` | 1 | new fragment |

## 5. What this does NOT do

- It carries no Rust, rows or web outcome: their read sets need a per-package census and a subset
  leg, and they run as before on #822.
- It records no per-mutant probe trace; a traced read set is the later refinement for #822 and
  every later release pull request.
- It admits no release pull request but #822; the text that would admit a later one, for SPEC-422
  and the release-legs ruling, is drafted for the owner's signature and not installed (#822).
- It changes no bound, price, timeout, ceiling, share or per-mutant budget, and no listing (#822).
- It waives no VOID and no FAIL: a carried survivor stays a FAIL until it is killed or recorded
  (#822).

## 6. Risks

- **A read set that misses a probed path carries a stale outcome.** This is the one way the gate
  could weaken. R7's allowlist is default-whole, and A2's planted controls hold each known escape
  (a root reader, a shadowing module, an absent probe, a binary section, the own-file rule). A new
  construct the allowlist does not name makes the file whole-tree, never bounded.
- **Determinism is assumed, not checked.** An outcome is taken as a function of its read set's
  contents and the key, on the runner image the jobs name by label; a flaky test or one near its
  timeout can differ between two draws, so `timeout` never carries and both draws stay one draw
  each.
- **A forged base.** A fork's run is never a base (R2), and a base's head must be an ancestor of
  this head; an artifact only its own run's jobs can upload.
- **An expired base** is no base, so the run is judged whole.
- **A yield near zero.** With R7's allowlist most population files are whole-tree at `75d555c4`,
  so a run after a change to any path carries little; a traced read set (section 5) is the remedy.

## 7. What only CI proves

The base steps and the downloads run only under the workflow token, so A3's `Workflow` test reads
`ci.yml`'s text and a release run's own log decides the rest: its `mutation-plan` log names the
base it chose or why there was none, and its verdict log prints, per Python class, the listed,
re-run and carried counts.

## 8. Formal

The union rule and the per-file carry decide the release gate's verdict, so a Lean entry,
`formal/lean/Formal/ReleaseCarry.lean`, ports `mutation_carry.py`'s union rule and carry predicate
branch for branch and covers both by their anchors. Its theorems, each entering at `ramp=report`:
`the_listing_is_rerun_plus_carried_once` (the re-run and carried sets the delivered code computes
partition the listing) and `the_verdict_reads_only_the_listing` (a verdict that reads outcomes only
on the listing gives the same answer on the merged outcomes as on outcomes all taken at the head,
when carried outcomes equal the head's). Its witness is the own-file-only predicate, which carries a
mutant whose read set changed. No covered span of another entry is edited.

## 9. Mutation rows (S49300-S49399)

The runner mutates `scripts/mutation_carry.py` and the verbs the delivery adds. The rows hold what
no generated mutant reaches, each killed by a test of `test_mutation_carry.py` `Workflow`:

| id | target | mutant | killed because |
|---|---|---|---|
| S49301 | `.github/workflows/ci.yml`, `mutation-plan`'s `actions: read` line | the line deleted | the job's permissions no longer read exactly R2's pair |
| S49302 | `ci.yml`, the base step's `continue-on-error: true` | `false` | a base step may fail the plan |
| S49303 | `ci.yml`, the `shards` step's `python-rerun.json` | `python-listed.json` | `shards` deals the listing, not the re-run set |
| S49304 | `ci.yml`, the manifest upload's `if: ${{ always() }}` | the line deleted | the manifest is not kept when a leg fails |
