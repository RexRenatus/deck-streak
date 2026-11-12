# SPEC-685: the agent documents name the CI jobs, gate stages and dispatch scopes the tree holds

- **Issue:** none. **Context(s):** `repo` (`docs/`, `scripts/tests/` and `CLAUDE.md`).
- **Decided by:** no ADR of its own: the delivery corrects documentation to the tree and decides
  nothing an ADR records. It cites ADR-055, ADR-057, ADR-069 and ADR-073, and amends none of them.
- **Status:** built. It holds `docs/red-first/SPEC-685.md`. **Mutation band:** none (section 9).

## 1. The problem, measured

AGENTS.md, CLAUDE.md and docs/BUILDER-BRIEF.md are written for agents, and they link six files
one hop: CHARTER.md, docs/CONTEXT-MAP.md, docs/LEXICON.md, docs/CODE_STYLE.md, docs/TESTING.md
and SECURITY.md. Every checkable claim of fact in the nine files (each path, command, job, stage,
branch, crate, reference and count) was read against the file it describes. Their sections and
placeholders are complete. Seven claims in three files are false, and each names a job, a stage or
a scope that the tree names differently:

- **docs/BUILDER-BRIEF.md:60.** "Six CI jobs judge every pull request, ... each a need of `ci`":
  the `ci` job of `.github/workflows/ci.yml` needs eight mutation jobs: `mutation-plan`,
  `mutation-rust`, `mutation-python`, `mutation-python-release`, `mutation-rows`,
  `mutation-verdict`, `mutation-web-shard` and `mutation-web`.
- **docs/BUILDER-BRIEF.md:65-66.** "`mutation-web` runs StrykerJS over every changed Mini App
  file": `mutation-web-shard` runs `stryker run`, one job per shard. `mutation-web` needs it,
  unites the shards' reports into one and judges.
- **docs/BUILDER-BRIEF.md:66-67.** The bullet's list of jobs leaves out `mutation-python-release`,
  which runs the Python legs that a release run deals past the first matrix.
- **CLAUDE.md:57.** The gate comment reads "fmt · clippy -D warnings · nextest · doctests · web ·
  oracle · scrub · secrets". `STAGES_ALL` in scripts/check.sh names twelve stages: fmt, clippy,
  test, doctest, audit-rust, test-engine, test-release, web, audit-web, python, scrub and secrets.
  No stage is named `oracle`.
- **docs/TESTING.md:75.** The CI table has a row "`packs` | `packs`", but no job of ci.yml and no
  stage of the gate is named `packs`. scripts/check.sh:29-30 says the packs are judged by
  `scripts/box-packs.sh`, never by the gate.
- **docs/TESTING.md:122-123.** "one CI job `mutation-python` per shard": the legs that a release
  run deals past that matrix run as `mutation-python-release` jobs.
- **docs/BUILDER-BRIEF.md:105-107.** "a dispatch of it with `package` set sweeps one crate, or
  `miniapp`": `.github/workflows/mutation-weekly.yml` compares its `package` input with `miniapp`
  and with `python`, and `python` runs the Python shards alone.

## 2. Requirements

- **R1.** The bullet in docs/BUILDER-BRIEF.md that lists the CI mutation jobs counts and names
  exactly the mutation jobs that `ci` needs in `.github/workflows/ci.yml`. There are eight, and
  `mutation-web-shard` and `mutation-python-release` are among them.
- **R2.** That bullet names the job that runs StrykerJS (`mutation-web-shard`, one job per shard).
  It also names the job that unites the shards' reports and judges (`mutation-web`), which needs it.
- **R3.** The gate comment in CLAUDE.md names every stage of `STAGES_ALL` in scripts/check.sh, in
  its order, and no other stage. It keeps `-D warnings` beside `clippy`.
- **R4.** The CI table in docs/TESTING.md:
  - names only jobs that ci.yml holds;
  - gives each job the stages that its `bash scripts/check.sh` line hands the gate;
  - holds the stages of `STAGES_ALL`, each once;
  - matches its count of jobs ("five") to the number of jobs that run the gate.

  The `packs` row goes.
- **R5.** The sentence in docs/TESTING.md on the Python mutation jobs names every job whose matrix
  is dealt from one of the plan's Python matrices: `mutation-python` and `mutation-python-release`.
- **R6.** The sentence in docs/BUILDER-BRIEF.md on the weekly dispatch names every value that
  `.github/workflows/mutation-weekly.yml` compares its `package` input with: `miniapp` and
  `python`.
- **R7.** The guard is `scripts/tests/test_agent_docs.py`, with one test per criterion. It reads
  each file as text from the repository root and runs no script or subprocess. It imports only
  `re`, `unittest` and `_support`. It reports `examined N` for each population it enumerates, and
  it refuses zero.
- **R8.** Each correction is the smallest edit that makes its claim true. Every other line of the
  three documents stays byte-equal, and no rule, voice or order changes. No workflow, script or
  production file changes.

## 3. Acceptance criteria of SPEC-685

| id | criterion | decided by |
|---|---|---|
| A1 | R1: the bullet's count word equals the number of mutation jobs `ci` needs, and the mutation jobs the bullet names are exactly those jobs | `test_agent_docs.py` `test_the_builder_brief_counts_and_names_every_mutation_job_ci_needs` |
| A2 | R2: the job the bullet says runs StrykerJS is each job with a `stryker run` line, and the job it says unites the shards' reports needs that job | `test_agent_docs.py` `test_the_builder_brief_names_the_job_that_runs_strykerjs` |
| A3 | R3: the first word of each item of the gate comment, in order, is `STAGES_ALL` | `test_agent_docs.py` `test_the_gate_comment_names_every_stage_check_sh_runs` |
| A4 | R4: every table job is a ci.yml job, each job's stages are its gate run's, the table's stages are `STAGES_ALL` each once, and its count word is the number of gate runs | `test_agent_docs.py` `test_the_ci_table_names_the_jobs_and_stages_ci_and_check_sh_hold` |
| A5 | R5: the jobs the sentence names are the jobs whose shard is dealt from a Python matrix of `mutation-plan` | `test_agent_docs.py` `test_the_coverage_section_names_every_python_shard_job` |
| A6 | R6: the scopes the sentence names are the values the weekly workflow compares `package` with | `test_agent_docs.py` `test_the_builder_brief_names_every_scope_the_weekly_dispatch_takes` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_builder_brief_counts_and_names_every_mutation_job_ci_needs
A2: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_builder_brief_names_the_job_that_runs_strykerjs
A3: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_gate_comment_names_every_stage_check_sh_runs
A4: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_ci_table_names_the_jobs_and_stages_ci_and_check_sh_hold
A5: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_coverage_section_names_every_python_shard_job
A6: python3 -m unittest discover -s scripts/tests -p test_agent_docs.py -k test_the_builder_brief_names_every_scope_the_weekly_dispatch_takes
```

Each test reads one claim of a document and the file that the claim describes, and it takes the
true value from that file when it runs. If a job, a stage or a scope changes and the documents do
not follow, the test turns red. Each test asserts what it found beside its examined count. R7 and
R8 are rules of the delivery, read in review.

## 4. File manifest

| path | context | change |
|---|---|---|
| `scripts/tests/test_agent_docs.py` | repo | added: the six tests (R1 to R7) |
| `docs/BUILDER-BRIEF.md` | repo | changed: the mutation bullet's count and jobs, and the weekly dispatch's scopes (R1, R2, R6) |
| `CLAUDE.md` | repo | changed: the gate comment names the twelve stages (R3) |
| `docs/TESTING.md` | repo | changed: the `packs` row goes, and `mutation-python-release` is named (R4, R5) |
| `docs/specs/SPEC-685-the-agent-documents-name-the-ci-jobs-gate-stages-and-dispatch-scopes-the-tree-holds.md` | repo | added: this SPEC |
| `docs/red-first/SPEC-685.md` | repo | added: the red-first record |
| `changelog.d/agent-docs-match-the-tree.md` | repo | added: the changelog fragment |

No register needs an entry for the new module (section 6).

## 5. What this does NOT cover

- Claims that the tree cannot decide are not read: the repository's visibility and rulesets, the
  probes and packs that run outside CI, and how a tool behaves outside the tree.
- The rules and intent the documents state are not read. Only claims of fact were read.
- AGENTS.md and the six linked files are not changed: none of them holds a false claim.

## 6. Risks

- **A workflow changes after this lands.** Each test takes the true value from the file when it
  runs, so a renamed, added or removed job that the documents do not follow turns its test red.
  If a `needs` list is written over several lines, the test reads it as empty, and `examined`
  refuses zero. The test then fails by name; it never passes after reading nothing.
- **A census reads the new module.**
  - `scripts/tests/test_ci_workflows.py` judges the imports and bare names of every module under
    `scripts/tests`. The new module imports only `re`, `unittest` and `_support`, and it uses only
    the builtins that census places. It imports no module of its read census, so it adds no read
    or dynamic site.
  - `scripts/tests/test_quarantine_census.py` reads every test file for skip markers. The new
    module holds none.
- **The documents change before the build.** The corrections are whole files made at one base.
  The build compares each document's blob with that base before it installs them.

## 7. What only CI proves

- The new module runs in the `hygiene` job of CI, which runs `scripts/tests`. It also runs
  anywhere by name, because it reads only text of the tree.

## 8. Formal model

No state, actor or write path is added. The module only reads six text files, and the documents
are prose. No model checker runs.

## 9. Mutation rows

None, and no band is claimed. No production file, guard script or census changes. Each of the six
tests is seen red at the red commit for its own claim (docs/red-first/SPEC-685.md).

## 10. References

ADR-055; ADR-057 and SPEC-039; ADR-069; ADR-073; SPEC-057 R13 and R14; SPEC-419 and ADR-433;
SPEC-422 R9 to R11.
