# SPEC-668: each crate name the workspace's lock holds more than once is listed with its reason, or reported

- **Issue:** #1389, a follow-up to SPEC-436 and ADR-450 from #913. **Context(s):** the workspace's
  dependency graph as `Cargo.lock` records it, and the guard tests under `scripts/tests/`; no
  crate's source, manifest, lockfile or `deny.toml` changes.
- **Decided by:** ADR-691 (a census of the lock's text beside cargo-deny's own warning; its
  forms; a listing keyed by name; warn first under ADR-450 D6).
- **Schematic:** none. No component, data flow or state machine of the service changes: one guard
  test reads one file the tree already holds.
- **Status:** delivered by the pull request that adds this file, in two pushes, red first.
  **Mutation band:** S668 (section 8).

## 1. The problem, measured

`deny.toml`'s `[bans]` table sets `multiple-versions = "warn"`, so cargo-deny's bans check, which
the `rust` job runs through `scripts/check.sh`'s `audit-rust` stage
(`cargo deny --locked check advisories bans licenses sources`), prints a warning for each crate it
finds at more than one version and refuses none. No SPEC states the rule, and nothing lists which
duplicates are held on purpose, so a new one reads the same as an old one.

A reading of the lock at the base found at least 37 names held by more than one `[[package]]`
entry: 36 at two or more versions, and `fsrs` at one version from two sources (the registry
release the engine takes and the upstream revision `deck-streak-fsrs7` takes, ADR-338). One of
them, `toml` 1.1.8, was read as reached from the workspace only through a dev-dependency. These figures
are a lower bound: the whole population is counted by the census itself, whose first run in CI
prints `examined <n> package(s) in the lock` and one line per site, and push 2 records that run
here: `hygiene` job JOB-NEW.

## 2. Requirements

R1. The census is `scripts/tests/test_lock_duplicates.py`. Its population is every `[[package]]`
    entry of the root `Cargo.lock`, read by `tomllib` for its name, version and source. It prints
    `examined <n> package(s) in the lock` and fails on an empty population, which is VOID.
R2. Entries are grouped by name. A name is a site when its entries hold two or more versions
    (`versions`), or one version from two or more sources (`sources`).
R3. A dependency's kind is not in the lock, so a version reached only through a dev or a build
    dependency stays in the population and is judged like any other.
R4. A dependency string is read in each of cargo's three forms, `name`, `name version` and
    `name version (source)`, with a git source's `#<commit>` dropped. Each version's dependents
    are the entries whose strings name it; a string that names no one entry prints
    `UNDECIDED duplicate-crate-version <entry>: <string>` and fails at both levels.
R5. `LISTED` holds each site of the lock at the delivery's base, by name, with the reason each
    version is there: its dependent and the workspace crate it is reached from. A listed site
    prints `LISTED duplicate-crate-version <name>: <versions>` and passes at both levels.
R6. An unlisted site prints `WARN duplicate-crate-version <name> (<form>): <version> <source kind>
    by <dependents>; ...` and passes at `warn`, and fails naming that line at `fail`.
R7. A listed name that is no site prints `STALE duplicate-crate-version <name>` and fails at both
    levels.
R8. `LEVEL` is `warn`. Moving it to `fail` is its own delivery, under ADR-450 D6's promotion rule;
    demoting it once at `fail` is a weakening.
R9. No lockfile, manifest, `deny.toml`, production file or existing test changes, and
    cargo-deny's `multiple-versions = "warn"` stays in force.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1, R2, R5: over the repository's lock, every site is listed or warned and no listing entry is stale or undecided | `test_lock_duplicates.TheWorkspaceLockHoldsOnlyListedDuplicates.test_every_duplicated_name_in_the_lock_is_listed_or_warned` |
| A2 | R8: `LEVEL` is `warn` | `test_lock_duplicates.TheWorkspaceLockHoldsOnlyListedDuplicates.test_the_census_starts_at_warn` |
| A3 | R2, R6: a planted name at two versions warns at `warn` and fails at `fail` | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_a_name_at_two_versions_warns_at_warn_and_fails_at_fail` |
| A4 | R2: a planted lock whose every name has one version reads clean at both levels | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_a_name_at_one_version_is_clean` |
| A5 | R5: a planted listed duplicate reads clean at both levels | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_a_listed_duplicate_is_clean_at_both_levels` |
| A6 | R2, R4, R6: a planted name at one version from two sources is a site | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_one_name_from_two_sources_is_a_site` |
| A7 | R7: a planted listing entry for a name held once fails at both levels | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_a_listed_name_no_longer_duplicated_is_stale_at_both_levels` |
| A8 | R1: a planted lock with no package entry is VOID | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_an_empty_lock_is_void` |
| A9 | R4: a planted dependency string that names no one entry fails at both levels | `test_lock_duplicates.TheCensusJudgesItsPlantedLocks.test_a_dependency_that_names_no_one_entry_is_undecided_at_both_levels` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_every_duplicated_name_in_the_lock_is_listed_or_warned
A2: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_the_census_starts_at_warn
A3: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_a_name_at_two_versions_warns_at_warn_and_fails_at_fail
A4: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_a_name_at_one_version_is_clean
A5: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_a_listed_duplicate_is_clean_at_both_levels
A6: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_one_name_from_two_sources_is_a_site
A7: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_a_listed_name_no_longer_duplicated_is_stale_at_both_levels
A8: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_an_empty_lock_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_lock_duplicates.py -k test_a_dependency_that_names_no_one_entry_is_undecided_at_both_levels
```

Each runs in the python stage of `scripts/check.sh`, which CI's `hygiene` job runs.

## 4. File manifest

- `scripts/tests/test_lock_duplicates.py`: the census, its listing and its tests (new).
- `scripts/tests/fixtures/lock-duplicates/site.lock.fixture`: one name at two versions (new).
- `scripts/tests/fixtures/lock-duplicates/clean.lock.fixture`: every name at one version (new).
- `scripts/tests/fixtures/lock-duplicates/listed.lock.fixture`: a listed duplicate (new).
- `scripts/tests/fixtures/lock-duplicates/two-sources.lock.fixture`: one version from two sources
  (new).
- `scripts/tests/fixtures/lock-duplicates/stale.lock.fixture`: a name held once (new).
- `scripts/tests/fixtures/lock-duplicates/empty.lock.fixture`: no package entry (new).
- `scripts/tests/fixtures/lock-duplicates/undecided.lock.fixture`: a bare string over two versions
  (new).
- `scripts/mutation-rows.d/S66800-S66899.json`: the census's rows (new).
- `docs/specs/SPEC-668-each-crate-name-the-lock-holds-more-than-once-is-listed-or-reported.md`: this
  file (new).
- `docs/decisions/ADR-691-duplicate-crate-versions-are-a-census-of-the-lock-at-warn-beside-cargo-deny.md`
  (new).
- `docs/red-first/SPEC-668.md`: the red-first record (new).
- `changelog.d/duplicate-crate-versions-1389.md`: the changelog fragment (new).

Each planted lock ends in `.fixture`, so no runner collects it; the census reads each as text.

## 5. What this does NOT do

- It cures no duplicate: no lockfile, manifest or `deny.toml` change; each site stays listed until
  its own change removes it (#1389).
- It changes no cargo-deny setting: `multiple-versions` stays `warn`, and raising it with a `skip`
  entry per listed name is a question for its own delivery (#1389).
- It reads no web lockfile: the rule's words are crate versions, and a widening to the web
  packages is its own item (#913).
- It moves no level to `fail`: that is its own delivery under ADR-450 D6 (#1389).

## 6. Risks

- A dependency update moves a listed name to one version: the real-lock test fails naming
  `STALE`, and the update drops that entry in the same pull request. Detected in the `hygiene`
  job.
- A dependency update adds a duplicate: the real-lock test prints `WARN` and passes at `warn`; the
  next listing change or the promotion delivery decides it. Detected by the line in the `hygiene`
  job's log.
- cargo writes a dependency string in a form the reader does not know: the real-lock test fails
  naming `UNDECIDED`, never a silent pass. Detected by the planted undecided lock and the real-lock
  test.
- The census and cargo-deny disagree on scope: cargo-deny leaves out a crate reached only through
  dev-dependencies, the census keeps it (R3). Read by comparing the `rust` job's bans lines with
  the census's lines at push 1.

## 7. What only CI proves

The real-lock test reads the repository's own lock, so its count and its lines are read in the
`hygiene` job of push 1, and push 2 lists any site that job names, with its job id, in section 1.

## 8. Mutation rows (S668)

Rows S66800 to S66813 in `scripts/mutation-rows.d/S66800-S66899.json`, table `SCRIPT_MUTATIONS`, each on
`scripts/tests/test_lock_duplicates.py`, each killed by a planted test of its own module, by full
id. Each find occurs exactly once in the file.
