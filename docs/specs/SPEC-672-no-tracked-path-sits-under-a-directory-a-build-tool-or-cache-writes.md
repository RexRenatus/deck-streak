# SPEC-672: no tracked path sits under a directory a build tool, package manager, test runner or cache writes

Issue #1391, a follow-up to SPEC-436 and ADR-450 from #913. ADR-695 decides it. Each figure below
names the command that produced it, run at the delivery's base.

## 1. The problem, measured

- The public scrub refuses a binary file by the rule `binary`, whatever its path (SPEC-033 R2), so
  a compiled Python cache is refused. A text file under a build or cache directory passes it, and
  no check names such a directory.
- A `.gitignore` keeps an untracked path untracked and does not affect a path git already tracks;
  `git add -f` adds an ignored path (ADR-039 rejected an ignore line as a guard for that reason).
  The root `.gitignore` names `mutants.out/`, and two tracked paths sit under one (SPEC-290 tracks
  them by force).
- The census reads every path `git ls-files` lists, each once. The design that drafted this SPEC read the
  tracked path list by a program that re-implements R2 to R6, and found four sites: the two
  recorded shard report files under a planted fixture's `mutants.out/` (SPEC-290), and the two
  built files under `vendor/deepmerge/dist/`, beside the vendored copy's `package.json`
  (SPEC-499 R2). That reading left a part of the list unread, so four is a lower bound, never the
  whole count: the census reads every tracked path, and its first CI run measures them all
  (section 7).

## 2. Requirements

R1. The census's population is every path `git ls-files` lists at the repository root, each
    examined once. It prints `examined <n> tracked path(s)`.
R2. A path is a site when one of its directories carries a name only a tool writes:
    `node_modules` (npm or pnpm), `__pycache__` (CPython), `.pytest_cache` (pytest), `.ruff_cache`
    (ruff), `htmlcov` (coverage.py), `.svelte-kit` (SvelteKit), `.vite` (Vite), `.build` and
    `.swiftpm` (SwiftPM), `DerivedData` and `xcuserdata` (Xcode), `.lake` (Lake), `mutants.out`
    and `mutants.out.old` (cargo-mutants), `.stryker-tmp` (StrykerJS), `test-results`,
    `playwright-report` and `blob-report` (Playwright) and `.check-logs` (the gate); or a name
    ending `.xcresult` (xcodebuild).
R3. A path is a site when its own name is a cache file's: a name ending `.pyc` or `.pyo`
    (CPython), `.coverage` alone or followed by a dot and a suffix (coverage.py), or a Vite
    configuration's name followed by `.timestamp-` (Vite).
R4. A directory named `target` is a site only where the directory holding it holds a tracked
    `Cargo.toml`, and one named `build`, `dist`, `coverage` or `reports` only where the directory
    holding it holds a tracked `package.json`. Anywhere else such a directory is a source tree and
    no site.
R5. A path is a site when it lies under a directory this tree's own configuration names as a
    tool's output: `web/app/src/lib/paraglide/` (Paraglide), `web/app/project.inlang/cache/`
    (inlang), an `ios/<name>.xcodeproj/` (XcodeGen), `ios/EnginePackage/DeckStreakFFI.xcframework/`
    (xcodebuild) and `ios/EnginePackage/Sources/DeckStreakFFI/` (UniFFI).
R6. A path's directories are read from the root down, and the first that R2, R4 or R5 names
    decides its tool and form; R3 is read only when none does. A file that names such a directory
    without lying under it, such as a `.gitignore`, is no site.
R7. At `LEVEL` warn an unlisted site prints `WARN build-output <path> (<tool>, <form>)` and
    passes; at fail it fails as `FAIL build-output <path> (<tool>, <form>)`.
R8. At both levels a listed path that is no site fails as
    `STALE build-output <path>: listed, but it is no site`.
R9. An empty population is refused, VOID, never a pass.
R10. `LISTED` holds today's sites, each a path with its tool, its form and the record that keeps
    it: the two shard report files of SPEC-290 and the two built files of SPEC-499's vendored copy.
    Over the repository each listed path is a site of its listed tool and form.
R11. `LEVEL` is warn. Moving it to fail is its own delivery, under ADR-450 D6's promotion rule;
    demoting it once at fail is a weakening.
R12. Each planted control is a path list read as text from a `.fixture` file under
    `scripts/tests/fixtures/build-output/`, never a directory, so a control adds no site of its
    own; a line opening `#` says why the file is planted.

## 3. Acceptance criteria of SPEC-672

| id | criterion | decided by |
|---|---|---|
| A1 | R2, R3, R4, R5: each planted site of `sites.fixture` is found with its tool and form, and the manifests planted beside them are no site | `test_build_output_census.PlantedPathListsAreJudged.test_a_path_under_a_directory_a_tool_writes_is_a_site` |
| A2 | R4, R6: each path of `clean.fixture` is no site, judged beside one planted site that is | `test_build_output_census.PlantedPathListsAreJudged.test_a_lookalike_or_a_source_tree_is_no_site` |
| A3 | R7, R8, R11: an unlisted planted site prints its exact `WARN` line at warn and fails naming it at fail, a listed one is clean at both, a stale entry fails at both, and `LEVEL` is warn | `test_build_output_census.PlantedPathListsAreJudged.test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails` |
| A4 | R1, R9, R12: `empty.fixture` plants no path, and the census refuses it naming `examined 0 tracked path(s)` | `test_build_output_census.PlantedPathListsAreJudged.test_an_empty_population_is_refused` |
| A5 | R1, R10: over the repository each listed path is a site of its listed tool and form, no entry is stale, and each unlisted site prints at warn | `test_build_output_census.TheRepositoryIsCensused.test_every_listed_path_is_a_site_of_its_form_and_the_rest_print_at_warn` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k a_path_under_a_directory_a_tool_writes_is_a_site
A2: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k a_lookalike_or_a_source_tree_is_no_site
A3: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails
A4: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k an_empty_population_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_build_output_census.py -k every_listed_path_is_a_site_of_its_form_and_the_rest_print_at_warn
```

Every planted path, expected tool, form and line is a literal the test writes, never a value the
census computes. Each test asserts its positive artifact (the found sites, the printed lines, the
refusal's message) before anything else.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_build_output_census.py` | the census | added: the reader, the forms, the listing, the level and the tests |
| `scripts/tests/fixtures/build-output/sites.fixture` | planted sites | added |
| `scripts/tests/fixtures/build-output/clean.fixture` | planted clean paths | added |
| `scripts/tests/fixtures/build-output/empty.fixture` | the empty population | added |
| `docs/specs/SPEC-672-no-tracked-path-sits-under-a-directory-a-build-tool-or-cache-writes.md` | this SPEC | added |
| `docs/decisions/ADR-695-tracked-build-output-and-caches-are-a-path-census-that-starts-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-672.md` | the red-first record | added |
| `changelog.d/test-build-output-census.md` | the fragment | added |
| `scripts/mutation-rows.d/S67200-S67299.json` | the rows of section 9 | added |

No `.gitignore`, workflow, script, existing test or production file changes, and no tracked path
is removed or moved.

## 5. What this does NOT cover

- Removing a listed site, or adding an ignore line for one: each changes a record that keeps the
  site on purpose (SPEC-290, SPEC-499), and stays with #1391.
- A generated file outside a build or cache directory, such as the compiled design tokens or a
  file the harness job places: #1391.
- A tool directory spelt in another case, a tool this tree does not run, and an output directory
  a setting moves to a place this tree's configuration does not name: #1391.
- A site in the part of the tracked path list the design did not read: it prints at warn,
  unlisted, until a delivery of #1391 lists or removes it.
- Moving `LEVEL` to fail, a delivery of its own under ADR-450 D6: #1391.
- The content check stays the public scrub's: its binary refusal (SPEC-033 R2) is unchanged, #1391.

## 6. Risks

- **A directory a package keeps on purpose reads as a site.** A shared name beside its tool's
  manifest is a site by R4 even when the tree keeps it, as the vendored copy's `dist/` is; it is
  listed with its record (R10), never exempted, and the listing makes each such choice visible.
- **Another census meets the new files.** A census over every test module or every fixture reads
  the new module and its three fixtures; the pull request's hygiene job runs every module of
  `scripts/tests`.
- **A listed path moves.** R8 fails the stale entry at once, so the move edits the listing.
- **`git ls-files` lists nothing or fails.** The subprocess raises, or R9 refuses: VOID either way.

## 7. What only CI proves

- Today's sites over every tracked path: the hygiene job of the delivery's first push runs the
  repository test, which prints one `WARN build-output` line for each unlisted site; that job is
  130638505747, and the second push lists each site it printed or records why one stays unlisted.
- `scripts/tests/test_ci_workflows.py` and every other census of `scripts/tests` over the new
  module and fixtures, read in the hygiene job by name.
- The rows of section 9, by the `mutation-rows` job.

## 8. Formal model

No state, actor or write path is added: the census is a pure reader of a path list. No
`@phx covers` line under `formal/` names a file this delivery adds, so no model is re-read and no
model checker runs.

## 9. Mutation rows (S67200-S67299)

Rows S67200 to S67243, in `scripts/mutation-rows.d/S67200-S67299.json`, table
`SCRIPT_MUTATIONS`, each on `scripts/tests/test_build_output_census.py`. Each find occurs exactly
once in that file, and each killer is a planted test named by its full id.

| rows | what each mutant removes | killer |
|---|---|---|
| S67200-S67218 | one directory name of R2, in R2's order | A1 |
| S67219 | R2's `.xcresult` suffix | A1 |
| S67220-S67224 | one cache-file form of R3: `.pyc`, `.pyo`, the `.coverage` name, its suffixed form, Vite's timestamp file | A1 |
| S67225-S67229 | the manifest one shared name of R4 needs: `target`, `build`, `dist`, `coverage`, `reports` | A1 |
| S67230 | R4's holding directory | A1 |
| S67231 | R4's manifest requirement | A2 |
| S67232-S67236 | one configured directory of R5, in R5's order | A1 |
| S67237 | the file names each directory holds | A1 |
| S67238 | R7's listed skip | A3 |
| S67239 | R7's choice of a line or a failure by level | A3 |
| S67240 | R8's stale arm | A3 |
| S67241 | R11's warn | A3 |
| S67242 | R9's refusal | A4 |
| S67243 | R12's comment skip | A4 |

`tracked_paths` carries no row: only the repository test reads it, in CI.
