# SPEC-126: the mutation verdict reads each report by name, so its layout never depends on how many artifacts exist

- **Wave:** W4. **Issue:** #351. **Context(s):** `repo` (`.github/workflows/ci.yml`, its tests and
  `docs/`).
- **Decided by:** ADR-126 (this SPEC's own: one download per single-artifact producer, and what that
  was chosen against), ADR-057 (the mutation jobs and the verdict) and SPEC-039 R3 and R18 (the
  plan, the shards and the verdict; it takes a dated amendment).
- **Status:** delivered. It waited in `docs/specs/planned/` from its own commit until its tests were
  green, and the delivery moved it to `docs/specs/` (ADR-016). It holds `docs/red-first/SPEC-126.md`.

## 1. The problem, measured

- **`actions/download-artifact` at the pinned v8.0.1 lays a download out by how many artifacts
  matched.** Its source (`src/download-artifact.ts`) extracts into `path` itself when the download
  is by `name`, when `merge-multiple` is true, or when exactly one artifact matched; only two or
  more matches, without `merge-multiple`, extract each into `path/<artifact name>`. Its README says
  the same: "This change also applies to patterns that only match a single artifact."
- **The verdict's one download is a pattern.** `mutation-verdict` downloads `pattern: mutation-*`
  into `reports` and then reads `reports/mutation-plan/plan.json`, `reports/mutation-rows/rows.json`
  and `reports/mutation-plan/whole.json`, and the shards from `reports/mutation-rust-shard-<i>/`.
- **The pattern also matches `mutation-web`, which the verdict does not wait for.** The verdict's
  `needs:` are `mutation-plan`, `mutation-rust` and `mutation-rows`. On a pull request that changes
  no Rust, `mutation-rust` and `mutation-rows` upload nothing (their report directories are never
  made), so when the verdict starts before `mutation-web` has uploaded, the pattern matches
  `mutation-plan` alone. The plan then lands at `reports/plan.json`, the judge reads no plan and
  exits `VOID no plan`, and a re-run of the failed jobs, which finds two matches, passes (#351).
- **The shards carry the same hazard, masked.** The plan sizes 1 to 256 shards. A pull request whose
  diff needs one shard uploads one `mutation-rust-shard-0` artifact, and a pattern that matches it
  alone extracts it flat, so `reports/mutation-rust-shard-0/mutants.out/outcomes.json` is absent.
  Today the plan's own artifact makes the pattern match two, which hides it.

## 2. Requirements

R1. **The verdict's report layout never depends on how many artifacts match.** The plan is
downloaded by `name:` into `reports/mutation-plan`; the rows' report is downloaded by `name:` into
`reports/mutation-rows`; the shards are downloaded by `pattern: mutation-rust-shard-*` with
`merge-multiple: true` into `reports`, and each shard's artifact carries its own top directory
`mutation-rust-shard-<i>/`, so one shard and many land at the same paths.

R2. **The verdict reads no artifact of a job outside its `needs:`.** `mutation-web` stays out of
`needs:` (it is the slowest job and the verdict has no use for its report), and no download step of
the verdict names or matches `mutation-web`.

R3. **A producer that uploaded nothing leaves the verdict's reading unchanged.** A `name:` download
of an artifact that was never uploaded fails the step, so the rows' download runs only when the plan
said the rows ran (`needs.mutation-plan.outputs.rows == 'true'`); otherwise `reports/mutation-rows/`
is absent, as it is today, and the judge reads the not-applicable case from the plan. No step
carries `continue-on-error`. The shards' pattern matching nothing is not an error.

R4. **A test pins the layout.** It reads the verdict's and the shard job's steps from `ci.yml`,
applies the pinned action's layout rule to every combination of shard count, rows present and web
present, and asserts the paths the judge reads.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | in every combination of 0, 1 and 3 shards, rows present or not and `mutation-web` present or not, the plan is at `reports/mutation-plan/plan.json`, the rows' report at `reports/mutation-rows/rows.json` when it exists, and each shard's outcomes at `reports/mutation-rust-shard-<i>/mutants.out/outcomes.json` | `test_verdict_download.py` `the_layout_holds_for_every_count_of_artifacts` |
| A2 | the layout with `mutation-web` present equals the layout without it, no verdict download names or matches it, and it is not among the verdict's `needs:` | `test_verdict_download.py` `the_verdict_reads_no_artifact_of_a_job_it_does_not_need` |
| A3 | with the rows never uploaded, no download step of the verdict fails and no file of `reports/mutation-rows/` exists; and no step carries `continue-on-error` | `test_verdict_download.py` `a_producer_that_uploaded_nothing_leaves_the_reading_unchanged` |
| A4 | the judge command lines still read the plan, the rows and the whole listing at the paths above | `test_verdict_download.py` `the_judge_reads_the_paths_the_downloads_lay_down` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_layout_holds_for_every_count_of_artifacts
A2: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_verdict_reads_no_artifact_of_a_job_it_does_not_need
A3: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k a_producer_that_uploaded_nothing_leaves_the_reading_unchanged
A4: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_judge_reads_the_paths_the_downloads_lay_down
```

The test emulates the pinned action's layout rule (quoted in section 1) in pure Python over the
steps it parses from `ci.yml`; it does not run GitHub. Each test prints how many scenarios it
examined and refuses zero. The live proof is the pull request's own `mutation-verdict` run, a
docs-only diff, whose download step logs are quoted in the pull request.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | repo | changed: R1 to R3, the verdict's downloads and the shard job's output directory |
| `scripts/tests/test_verdict_download.py` | repo | added: A1 to A4 |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: the assertion that read `pattern: mutation-*` now reads the three downloads |
| `docs/specs/SPEC-126-the-mutation-verdict-reads-each-report-by-name.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-126-the-verdict-downloads-each-single-artifact-producer-by-name.md` | repo | added |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | repo | changed: a dated amendment at its end (insert-only) |
| `docs/red-first/SPEC-126.md` | repo | added |
| `changelog.d/fix-verdict-download-126.md` | repo | added |

No schematic: the change adds no component. The data flow is the one SPEC-039 draws (a job uploads
a report, the verdict downloads it), and section 2 says where each report lands.

## 5. What this does NOT do

- It changes no Rust and no Python outside the tests, because the defect is in a workflow's download
  layout (#351).
- It does not touch `mutation-weekly.yml`: its survivors job downloads every artifact of its own
  run and reads them by recursive search, which does not depend on the layout (#351).
- It adds no mutation-row band: the invariant lives in a workflow and is pinned by A1 to A4, and no
  row selects a workflow target (#351).
- It gives the reader no tolerance for a second layout, because a reader that accepts both layouts
  hides the defect (ADR-126, #351).
- It does not add a Python mutation job; the open pull request that adds `mutation-python` follows
  the same rule for its shards when it lands (#351).

## 6. Risks

- **The shard artifact's shape changes.** `mutation-rust` writes each shard under
  `shards/mutation-rust-shard-<i>/` and uploads `shards/`. Anything that reads the pull-request
  shard artifacts by path breaks; none does in the tree (A1 reads the layout the judge reads).
- **A `name:` download of a missing artifact fails the step.** R3 guards the rows' download with the
  plan's own output. If the rows job crashes before it makes its report directory, the download
  fails loudly instead of the judge reading VOID; both are red.
- **The emulation is not GitHub.** It applies the rule read from the pinned source; the pull
  request's live run is the check that the rule was read right.

## 7. References

Issue #351; SPEC-039 R3 and R18; ADR-057; ADR-016; ADR-126; `actions/download-artifact` v8.0.1
(`src/download-artifact.ts`, `README.md`).

## 8. Amendment, 2026-09-29: A4 reads each judge line, and two statements corrected

Insert-only under ruling (i) of SPEC-038 section 8: every earlier byte is kept in order. It inserts
this section and nothing else. Issue #358.

- **A4 now asserts per judge line.** The section 3 row and fenced command for A4 stand. The test
  parses the verdict job's `mutation-verdict.py judge` command lines, identifies each by its
  `--class` value, and asserts on that line alone: the `rust` line carries `--plan`,
  `--shard-reports`, `--rows` and `--whole` at the paths section 3 names, and the `oracle` line
  carries `--plan` and `--rows` there. It refuses zero judge lines and prints how many it examined.
  A judge line of a class the test does not name is neither asserted nor an error, so a later
  class does not redden it. Before this amendment A4 asserted each path as a substring of the whole
  job, which passes when a flag moves from one judge line to the other or is dropped from one line
  while another still carries it. Two plants, recorded in `docs/red-first/SPEC-126.md`, are killed:
  `--plan` removed from the `rust` line while the `oracle` line carries it, and `--rows` removed
  from the `oracle` line while the `rust` line carries it (#358).
- **Section 5's `mutation-weekly` sentence is corrected.** It read "its survivors job downloads
  every artifact of its own run and reads them by recursive search, which does not depend on the
  layout". The corrected sentence is: the survivors job downloads every artifact of its own run into
  `reports/<artifact name>/`. Its `survivors` command finds every `outcomes.json`, `mutation.json`
  and `rows.json` by recursive search below `reports/`. Its battery step runs two commands:
  `battery` reads `reports/mutants-shard-<i>/` (the `mutants.out/outcomes.json` and
  `cargo-mutants.exit` inside), `reports/rows/rows.json` and `reports/listing/whole.json` by name
  and the Stryker sweep by search below `reports/stryker`, and `table` reads
  `reports/listing/whole.json` by name and finds the shards and the Stryker report by recursive
  search. So the battery depends on the layout, which this SPEC's change to `ci.yml` does not
  alter (#358).
- **Section 3's "a docs-only diff" is corrected.** The pull request that delivered this SPEC changed
  a workflow and Python tests, so its diff was not docs-only; the corrected sentence is that the
  live proof is the pull request's own `mutation-verdict` run, on a diff that changed a workflow
  and Python, whose download step logs are quoted in the pull request (#358).

## 9. Amendment, 2026-09-29: the judge-line reader splits words as the shell does

Insert-only under ruling (i) of SPEC-038 section 8: every earlier byte is kept in order. It inserts
this section and nothing else. Issue #374.

- **The rule.** Section 8's reader took each word of a judge command as written, so a path the
  shell builds identically but spells differently was refused. The reader now splits each command
  into words as the shell does: it joins a backslash-newline continuation first, removes quotes
  (Python's `shlex.split` in POSIX mode, which removes the quotes it parses), and splits
  `--flag=value` into the flag and its value, as the verdict script's argument parser reads it.
  The `--class` value picks the line, as before.
- **An expansion is kept distinct from text.** `shlex` removes the quotes and the backslash, so it
  cannot tell `"$reports"` from `'$reports'`. A pass before it marks a `$` inside single quotes, or
  escaped by a backslash, as text, and a `$` outside every quote as unquoted. `"$reports"`,
  `"${reports}"`, `"$reports"/...` and `"$reports"'/...'` all read as the expansion of `reports`;
  `'$reports'` and `\$reports` read as the text `\$reports`; an unquoted `$reports` reads as
  `(unquoted)$reports`, which the test refuses because the shell word-splits it.
- **The command ends where the shell ends it.** The reader stops at a control operator (`;`, `&` or
  `|`) or at a `#` that starts a word, outside every quote, and the command's words stay on one
  line: a line break after `judge` with no backslash is not read as a space. A flag that only a
  comment or a later command carries is therefore not a flag of the judge line.
- **Every wrong path is still refused.** A single-quoted or escaped path, a different directory, a
  different variable, an unquoted expansion, a flag moved to the other judge line and a flag dropped
  from one line each stay refused. Three more are refused: a flag only in a comment, a flag after
  a control operator, and a line break after `judge` with no backslash (eleven wrong paths in all).
- **Two checks join A4, both in `test_verdict_download.py`.** The test
  `test_every_spelling_the_shell_reads_alike_passes` builds the verdict job with each of the seven
  spellings issue #374 names (braces, the expansion closed before the slash, a single-quoted tail,
  a quoted flag name, a backslash continuation, and `--flag=value` with the value quoted or with
  its quotes closing early) and asserts the judge-line check passes. The test
  `test_every_path_the_shell_reads_differently_is_refused` asserts the check refuses each path the
  shell reads differently, and `test_an_expansion_is_kept_apart_from_text_and_from_an_unquoted_one`
  pins how the reader tells the three kinds of `$` apart. They are run by
  `python3 -m unittest discover -s scripts/tests -p test_verdict_download.py`, and the red-first
  record names them A5 and A6.
- It changes no Rust, no workflow and no Python outside the test, because the defect is in the
  test's reader (#374).
- It adds no mutation-row band, for the reason section 5 gives (#374).

## 10. Acceptance criteria added by the section 9 amendment

| id | criterion | decided by |
|---|---|---|
| A5 | each of the seven spellings issue #374 names passes the judge-line check | `test_verdict_download.py` `every_spelling_the_shell_reads_alike_passes` |
| A6 | each path the shell reads differently is refused | `test_verdict_download.py` `every_path_the_shell_reads_differently_is_refused` |

```acceptance
A5: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_every_spelling_the_shell_reads_alike_passes
A6: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_every_path_the_shell_reads_differently_is_refused
```

## 13. Amendment, 2026-09-30: a report counts only in its own shard's slot

Insert-only under ruling (i) of SPEC-038 section 8: every earlier byte is kept in order. It inserts
this section and section 14 and nothing else. Issue #438.

- **The class rule.** The verdict counts a shard's work only from the report bound to that shard.
  It reads the report in `mutation-python-shard-<k>` only when the report's own `shard` field equals
  `<k>/<count>`, the slot it sits in, and only when the mutants the report examined equal the plan's
  listing for shard `<k>`. Any other layout is VOID, and the refusal names the shard.
- **What was measured.** The verdict checked that each slot held a readable report of the runner's
  schema and that no restore failed, and nothing more. A copy of one shard's report laid in another's
  slot, two reports swapped, a report of a shard field absent or malformed, a report trimmed of a
  mutant and a report carrying a mutant the plan never listed were all read and counted, so the
  verdict could pass on work no shard had done. Over three plans of two, three and four shards the
  generated population of such layouts held 440 members; the verdict accepted 431 of them.
- **The change.** `python_reports` in `scripts/mutation-verdict.py` refuses a report whose shard
  field is not the slot's `<k>/<count>`, and, through `shard_listing_drift`, one whose examined
  mutant names, as a multiset, differ from the plan's list for that shard: a missing mutant, an
  extra one, a repeat and a swapped one each differ. A plan that lists no mutants for the shard
  refuses the report as well.
- **Nothing else moves.** The runner, the plan, the workflow and the Rust lane are unchanged; a
  correct layout is judged as before.
- **Two checks join the SPEC, both in `test_mutation_python_shard_binding.py`.** Each generates its
  population from a plan's shards, prints the member count and the refused count, and asserts that
  the correct layout passes, that every other member is refused with exit 3 naming exactly the wrong
  slots, and that none is accepted. Eight mutation rows, `S12600` to `S12607`, one per arm of the
  rule, are proved by killers among them.

## 14. Acceptance criteria added by the section 13 amendment

| id | criterion | decided by |
|---|---|---|
| A8 | a report counts only in the slot its own shard field names: a copy, a swap and an absent or malformed field are each refused by name | `test_mutation_python_shard_binding.py` `test_a_report_counts_only_in_its_own_slot` |
| A9 | a report counts only the mutants the plan lists for its shard: a trimmed, extra, repeated, swapped or missing report is refused by name | `test_mutation_python_shard_binding.py` `test_a_report_counts_only_the_mutants_its_shard_lists` |

```acceptance
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_report_counts_only_in_its_own_slot
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_report_counts_only_the_mutants_its_shard_lists
```
