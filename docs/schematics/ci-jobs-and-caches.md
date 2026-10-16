# Schematic: the CI jobs, and the caches each one reads and writes

Kind: component and data flow. Read at DeckStreak `dev` 8f91667 (`.github/workflows/ci.yml`,
`scripts/check.sh`, `scripts/pack-rows.py`), and again at `dev` 63e6671 for SPEC-038's amendment
(section 8), which added the `engine` job. Decided by ADR-055; built by SPEC-038.

## The jobs

```mermaid
flowchart LR
  event{{a pull request into dev or main, or a push to dev or main}} --> rust
  event --> engine
  event --> web
  event --> packs
  event --> hygiene
  event --> lint[workflow-lint: zizmor]
  event --> base[base-is-dev]
  subgraph gate[check.sh stages, each in exactly one job]
    rust[rust: fmt clippy test doctest audit-rust]
    engine[engine, two slices: test-engine, one slice each]
    web[web: web audit-web]
    packs[packs: packs, a bounded pool of pack rows]
    hygiene[hygiene: python scrub secrets]
  end
  rust --> ci{ci: every need succeeded}
  engine --> ci
  web --> ci
  packs --> ci
  hygiene --> ci
  lint --> ci
  base --> ci
  ci --> required([the required check ci])
```

The five gate jobs need nothing, so they start together; the run's wall time is the slowest of
them plus the aggregate. Each calls `bash scripts/check.sh <its stages>` once. `hygiene` and `packs`
check out the whole history (`fetch-depth: 0`): `secrets` scans every commit, `scrub` reads every
blob reachable from `HEAD`, and the sdd numbering row reads every branch. `rust`, `engine` and `web`
check out one commit.

## The engine set, split between two stages

```mermaid
flowchart LR
  def[["ENGINE_TESTS in check.sh: the engine set E, defined once"]]
  def -->|"-E 'not (E)'"| test[test stage, in the rust job]
  def -->|"-E 'E'"| eng[test-engine stage, in the engine job]
  ws[(every test of the workspace)] --> test
  ws --> eng
  test --> one([each test runs in exactly one of the two])
  eng --> one
```

Both stages run `cargo nextest run --workspace --locked --no-fail-fast`, and differ only in the
filterset, so their build scopes, features and flags are one; E negated and E itself cover the
workspace once. E holds whole test binaries, SPEC-022's `sync` and `engine_budget`, so a test added
to either goes with it. The local gate with no arguments runs both stages.

In CI the `engine` job runs E in two slices, a matrix leg each: a leg hands `test-engine`
`ENGINE_SLICE=<m>/<N>` (N is the matrix's size) and the stage adds `--partition slice:<m>/<N>`,
nextest's round robin over the one list E selects, so the slices hold every test of E once.
Without `ENGINE_SLICE` the stage runs the whole set.

## The caches

```mermaid
flowchart TB
  subgraph push[a push to dev or main]
    prestore[restore] --> pstages[the stages]
    pstages --> phit{exact key hit?}
    phit -->|no| pclean[rust only: cargo clean --workspace]
    pclean --> psave[actions/cache/save]
    phit -->|yes| pnone[save nothing]
  end
  subgraph pr[a pull request, from this repository or a fork]
    rrestore[restore] --> rstages[the stages]
    rstages --> rnone[save nothing, ever]
  end
  psave --> scope[(the cache scope of dev or main)]
  scope -->|read by later pushes and by pull requests into that branch| prestore
  scope --> rrestore
```

| cache | paths | key | restored by | saved by |
|---|---|---|---|---|
| Rust | `~/.cargo/registry/index/`, `~/.cargo/registry/cache/`, `~/.cargo/git/db/`, `target/` | `rust-<os>-<hash of rust-toolchain.toml>-<hash of Cargo.lock>`, falling back to the same toolchain | `rust`; `engine`, which builds the same workspace scope for its stage; and `hygiene`, whose guard tests build a Rust example | `rust` only, on a push that missed, after `cargo clean --workspace`: two jobs saving one key race |
| pnpm store | the path `pnpm store path` prints | `pnpm-<os>-<hash of pnpm-lock.yaml>`, falling back to any | `web` | `web`, on a push that missed |
| Playwright's browser | `~/.cache/ms-playwright` | `playwright-<os>-<the locked Playwright version>-chromium-headless-shell` | `web` | `web`, on a push that missed, right after the install |

A pull request restores from its base branch's scope and from `main`'s; GitHub confines anything a
pull request might write to its own merge ref, and the workflow never writes from one: every
`actions/cache/save` step is conditioned on a push to `dev` or `main`.

The settle census (SPEC-072 A12, section 14) has no cache of its own, and restores nothing a
verdict reads. Each census compiles in an empty target directory under `target/tmp/settle-census`,
made for that census and removed when it ends, so the Rust cache's `target/` can carry nothing
there that a later census reads, and every census compile in the `rust` job is cold.

```mermaid
flowchart LR
  tree[the tree judged] --> meta[cargo metadata, locked and offline]
  meta --> owner{the member at crates/progression/Cargo.toml, unique?}
  owner -->|no| refuse[refused by name]
  owner -->|yes| empty[an empty target of its own]
  empty --> passes[the census's passes, an environment it names]
  passes --> verdict[the verdict]
  empty --> removed[removed when the census ends]
```

## One stage, in whichever job runs it

```mermaid
stateDiagram-v2
  [*] --> tools: check.sh names the stage
  tools --> failed: a tool is missing (FAILED, "missing tool: <tool> (<install hint>)")
  tools --> running: every tool present
  running --> ok: exit 0
  running --> failed: exit non-zero
  ok --> timed
  failed --> timed
  timed --> [*]: one summary line, and a row in timings.tsv
```

The stage logs and `timings.tsv` go to `${{ runner.temp }}/check-logs`, outside the checkout, and
each job uploads them as its own artifact, whatever the verdict.

Amendment (2026-09-28): SPEC-056 removed the `packs` job and stage this schematic draws, so
four gate jobs run; every pack is judged on the maintainer's box run (ADR-069).

## The mutants step, inside its memory scope, and the verdict's reading of it (SPEC-196, ADR-199)

A `mutation-rust` leg, a weekly rust leg and the `rehearsal` run `cargo mutants` through `scripts/memory_scope.py`, with the
arguments they ran before. The script puts its own process in a transient scope, checks the cap from inside, and runs
cargo-mutants as its child, so the command keeps the caller's user, groups and environment.

```mermaid
flowchart TD
  leg["a mutation-rust leg, a weekly rust leg or the rehearsal"] --> ms["python3 scripts/memory_scope.py --report DIR -- cargo mutants ARGS"]
  ms --> cap["the cap: MemTotal from /proc/meminfo x 15/16, whole pages"]
  cap --> unitcall["sudo busctl call StartTransientUnit: a scope holding the script's own process; MemoryMax = cap, MemorySwapMax = 0, OOMPolicy = continue"]
  unitcall --> check{"from inside: the unit holds the script; memory.max = cap; memory.swap.max = 0; memory.oom.group = 0; oom and oom_kill = 0; OOMPolicy = continue"}
  check -->|"no"| refused["REFUSED: exit 78, nothing run; record in_force false with the reason"]
  check -->|"yes"| begun["record: in_force true, state running"]
  begun --> cm["cargo mutants ARGS, the script's child, unchanged"]
  cm --> base["the unmutated baseline: build and test, under the cap"]
  base --> each["each mutant: build, then its tests under nextest"]
  each -->|"a test grows to the cap"| kill["the kernel stops the largest process in the scope, the runaway test; the scope and cargo-mutants go on"]
  kill --> status["nextest: the test's status reads SIGKILL; cargo-mutants records the outcome and tests the next mutant"]
  each --> report["mutants.out: outcomes.json and one log per scenario, as before"]
  status --> report
  report --> done["record: state done; oom, oom_kill and max counts; peak as a percentage of the cap"]
  done --> exit["the script exits with cargo-mutants' own exit, which the step records as before"]
```

```mermaid
flowchart TD
  shard["each shard the run promised, in judge, battery and table"] --> has{"a report or a cargo-mutants.exit?"}
  has -->|"no"| before0["judged as before"]
  has -->|"yes"| rec{"memory-scope.json present, readable and done?"}
  rec -->|"no"| void1["VOID by name: a cap kill cannot be excluded"]
  rec -->|"in_force false"| void2["VOID by name, with the scope's reason"]
  rec -->|"yes"| touched{"oom or oom_kill above 0?"}
  touched -->|"no"| before["judged exactly as before; no log read"]
  touched -->|"yes"| place["place kills: distinct tests whose nextest status is SIGKILL, in each scenario log"]
  place --> agree{"placed kills = oom_kill?"}
  agree -->|"no"| ambiguous["FAIL: MEMORY-CAP AMBIGUOUS, both counts and every placed scenario; no mutant scored"]
  agree -->|"yes, in a mutant"| score["score_memory_cap: FAIL MEMORY-CAP mutant; not examined"]
  agree -->|"yes, in the baseline"| baseline["FAIL: MEMORY-CAP the unmutated baseline"]
  score --> rest["every other outcome in the shard judged as before; examined = the tool's count less the named mutants"]
  rest --> unchanged["unchanged: a missing or partial report VOID; the partition of listed against tested; zero examined VOID"]
```

The scope holds the whole cargo-mutants tree, so the kernel's out-of-memory choice is made among our processes, the largest
first, before the machine is under pressure; the runner is outside it. `OOMPolicy=continue` leaves `memory.oom.group` at `0`,
so one process is stopped, never the scope. A listing (`cargo mutants --list`) runs no test and is not wrapped. A mutant the
cap stops is never caught and never a timeout: its leg fails naming it, and every other result in the leg stands. How a
named kill is scored is decided in `score_memory_cap` alone. The timeouts, the listing, the partition and the baseline are
the ones in the sections above; the scope adds a bound on memory and changes no bound on time.

## The dynamic-import register: before the push, and the census in the hygiene job (SPEC-406, ADR-420)

Kind: data flow. Read at DeckStreak `dev` 32f6217 (`scripts/tests/test_ci_workflows.py`,
`.github/workflows/ci.yml`, `scripts/check.sh`, `docs/specs/_TEMPLATE.md`). Decided by ADR-420;
built by SPEC-406.

A module under `scripts/tests` that loads code through `importlib`, `runpy` or `exec` is held to
the `DYNAMIC_IMPORTS` register twice: by `scripts/dynamic-imports-check.py` before the push, at the
module, and by the census in the `hygiene` job after it, at every site with its count. The check
reads only what `HEAD` commits; the census reads the checked-out tree.

```mermaid
flowchart TD
  added["a commit adds or changes scripts/tests/NAME.py"] --> pre["python3 scripts/dynamic-imports-check.py --base BASE"]
  pre --> regread["git show HEAD:scripts/tests/test_ci_workflows.py, parsed: the one DYNAMIC_IMPORTS assignment"]
  regread --> regshape{"a dict display of allowed groups, each site a tuple led by a module string, one module or more?"}
  regshape -->|"no"| void["VOID: exit 2, the cause named; nothing judged"]
  regshape -->|"yes"| regline["register: N module(s) in DYNAMIC_IMPORTS at HEAD"]
  regline --> changed["git diff --name-only --no-renames --diff-filter=AM BASE...HEAD -- scripts/tests: the .py paths"]
  changed --> anychange{"any changed module?"}
  anychange -->|"no"| na["NOT-APPLICABLE: exit 0, examined 0"]
  anychange -->|"yes"| parsed["each path by git show HEAD:PATH, parsed; a module that does not parse is VOID"]
  parsed --> loaderq{"a load of a name an importlib or runpy import binds, or a bare exec the module does not bind?"}
  loaderq -->|"no: the non-loader path"| nonloader["counted, not loader-style"]
  loaderq -->|"yes: loader-style"| memberq{"does the register name the module's dotted name?"}
  memberq -->|"yes: the registered path"| registered["counted, admitted"]
  memberq -->|"no: the unregistered path"| refused["REFUSED: exit 1, the path, the loader and its line; no push until the register names the module"]
  nonloader --> okline["examined M changed module(s), L loader-style; OK, exit 0"]
  registered --> okline
  okline --> pushed["the push"]
  na --> pushed
  pushed --> hygiene["the hygiene job: bash scripts/check.sh python scrub secrets"]
  hygiene --> census["the census: every site in every scripts/tests module, by module, qualified name and text"]
  census --> sums{"each site's count equals what NOT_WORKFLOW_READS and DYNAMIC_IMPORTS list?"}
  sums -->|"yes"| green["hygiene green; the aggregate ci needs it"]
  sums -->|"no"| red["hygiene red, naming the site"]
```

| path | what the check reads | its verdict | what the census then reads |
|---|---|---|---|
| registered | a changed module whose syntax tree loads code, and its dotted name among the register's modules | counted, `OK` | each of its sites against the register's count for it |
| unregistered | a changed module whose syntax tree loads code, and no tuple of the register naming it | `REFUSED`, exit 1, before any push | nothing yet: the push waits |
| non-loader | a changed module with no such load, the loader's words in a string or a comment included | counted, `OK` | its sites, if any other dynamic name is among them |

Every site the check counts is one the census counts: `importlib` and `runpy` are not in
`VETTED_MODULES` (`test_ci_workflows.py:4149`), and `exec` is in `BARE_DYNAMIC` (4069-4080). So the
check refuses a module only where the census would refuse it as well, and the census stays the
judge of a registered module that gains a site, of each tuple's text and count, and of every other
dynamic name. The register is the dict at `test_ci_workflows.py:6482-7064`, each group built by
`allowed()` (4779-4782); a module is named as `module_sources()` names it (4510-4518). The census
runs in `stage_python` (`scripts/check.sh:179-195`), the step at `.github/workflows/ci.yml:347-351`
of the `hygiene` job (line 298), which the aggregate `ci` needs (lines 875-878).

The SPEC template's file manifest section (`docs/specs/_TEMPLATE.md`, section 4) tells a delivery
that adds such a load to list `scripts/tests/test_ci_workflows.py` as changed, and names the check.

## The mutant leg's fetch before its baseline, and how a red baseline reaches the verdict (SPEC-416, ADR-430)

Kind: data flow. Read at DeckStreak `dev` c74b6b9 (`.github/workflows/ci.yml:65-66`, `:187-188`,
`:447-524`, `:485-496`, `:516-518`, `:622-624`, `:682-699`, `:904`; `scripts/mutation-verdict.py:133-140`,
`:1134`, `:1203-1206`, `:1260`; `crates/progression/tests/xp_census.rs:715`). Decided by ADR-430;
built by SPEC-416.

A `mutation-rust` leg of a pull request's run restores the Rust cache by key (`ci.yml:485-496`) and then
runs cargo-mutants (`:507-518`), whose first act is the unmutated baseline: the workspace's tests,
unmutated. One of those tests, the progression census, asks `cargo metadata --locked --offline` for the
graph (`xp_census.rs:715`). Offline, cargo reads only the packages already in the registry cache. A pull
request into `main` restores no cache written on `dev`, and a build downloads only the packages it
compiles, so the census can ask for a package no earlier step fetched. The `rust` job (`ci.yml:65-66`),
the `release` job (`:187-188`) and the `mutation-rows` job (`:622-624`) run a fetch after their restore;
`mutation-rust` (`:447-524`) runs none.

Before: a cache miss leaves the baseline to fail, and the leg still uploads a report.

```mermaid
flowchart TD
  restore["restore the Rust cache by key (ci.yml:485-496)"] -->|"a pull request into main: cache miss"| empty["an empty registry"]
  empty -->|"no fetch step in mutation-rust"| baseline["cargo-mutants: the unmutated baseline, build then test (ci.yml:507-518)"]
  baseline -->|"the progression census runs cargo metadata --locked --offline"| census["the census: failed to download a package"]
  census -->|"a red test, so a red baseline"| exitfour["cargo-mutants exits 4, 0 mutants examined"]
  exitfour -->|"rc captured and echoed (ci.yml:516-518)"| greenstep["the step is green; the report and cargo-mutants.exit are uploaded"]
  greenstep -->|"the verdict job downloads every leg's report"| partial["partial_reason: exit 4 is outside WHOLE_EXITS (mutation-verdict.py:133-140, :1203-1206)"]
  partial -->|"whole_reports records it (mutation-verdict.py:1260)"| void["the leg is VOID by name"]
  void -->|"Verdict.close returns EXIT_VOID (mutation-verdict.py:1134)"| judge["the judge step exits with its status (ci.yml:682-699)"]
  judge -->|"the aggregate exits 1 unless the result is success (ci.yml:904)"| cifail["the aggregate ci job fails"]
```

After: one fetch step sits between the restore and cargo-mutants, under the same `if:` as the cargo-mutants
step, so the baseline sees every target's packages and is red only when a test is red.

```mermaid
flowchart TD
  restore2["restore the Rust cache by key (ci.yml:485-496)"] -->|"hit or miss"| fetch["cargo fetch --locked --manifest-path Cargo.toml, under the cargo-mutants step's if"]
  fetch -->|"every target's packages are in the registry"| baseline2["cargo-mutants: the unmutated baseline"]
  baseline2 -->|"the census reads the graph offline and finds every package"| green["the baseline is green"]
  green -->|"mutants are tested"| examined["mutants examined in the leg"]
  examined -->|"the leg's report and exit are uploaded"| judge2["the verdict judges the examined count (unchanged)"]
  fetch -->|"a registry outage: the step fails"| noreport["no report is uploaded"]
  noreport -->|"a missing report is VOID by name"| judge2
```

The leg's colour stays green by design: its step records cargo-mutants' exit instead of returning it
(`ci.yml:516-518`), and the verdict reads that exit from the leg's report. So the verdict is where a red
baseline fails closed, at every step from the report to the aggregate (ADR-430 D3), and this delivery
changes no line of `scripts/mutation-verdict.py`. The fetch step changes no test: the census keeps
`--locked --offline`, and no test is skipped or filtered. The rows job's own fetch line (`ci.yml:622-624`)
stays the only line of its kind, because the new step's line is the `rust` and `release` jobs' own. The
weekly battery's `rust` leg and its rehearsal are not changed here; they stay open under the issue SPEC-416
refers to.

## The python stage's ruff arm and the computed-root guard (SPEC-424, ADR-438)

Kind: state machine and data flow. Read at DeckStreak `dev` ac7ebd7 (`ruff.toml:6-15`,
`scripts/check.sh:31`, `:77-81`, `:179-196`, `.github/workflows/ci.yml:298-358`,
`scripts/tests/test_check_gate.py:22-43`). Decided by ADR-438; built by SPEC-424.

Before this delivery, the python stage checks `python3` and `cargo`, runs its three suites and
prints one line of their results; nothing lints. After it, the stage also checks `ruff`, and runs
the ruff arm after the suites, whatever they found. The `hygiene` job installs that ruff from the
hashed pin first.

```mermaid
stateDiagram-v2
  [*] --> tools: check.sh names the python stage
  tools --> failed: python3, cargo or ruff missing (FAILED, missing tool and its install hint)
  tools --> suites: every tool present
  suites --> arm: scripts/tests, tools/parity-oracle and agent/tests each ran, red or green
  arm --> void: off its pin, no pin, an exit of 2, no file listed, or a count unread or unequal
  arm --> red: a lint finding or a format change (exit 1, counts named)
  arm --> clean: both arms exit 0 over the same count
  void --> failed: last line ends with ruff VOID and the reason
  red --> failed: last line ends with both arms' counts and exits
  clean --> judged: last line ends with both arms' counts and exits
  judged --> ok: every suite ran a test and exited 0
  judged --> failed: a suite exited non-zero or ran no test
  ok --> [*]
  failed --> [*]
```

The install, in the `hygiene` job, before the gate step:

```mermaid
flowchart TD
  pin["scripts/ruff-requirements.txt: ruff==version and a sha256 per wheel"] --> venv["a new virtual environment under the runner's temporary directory"]
  venv --> pip["pip install --require-hashes --only-binary :all: -r scripts/ruff-requirements.txt"]
  pip -->|"a digest the pin lacks, or a source archive"| refused["the step fails: the job is red before any stage"]
  pip -->|"every digest matches"| onlyruff["a directory holding only that ruff, appended to the job's path"]
  onlyruff --> gate["bash scripts/check.sh python scrub secrets"]
  toml["ruff.toml: required-version equal to the pin"] --> gate
```

| reading | what the arm saw | the stage's last line ends with | the stage |
|---|---|---|---|
| missing | no `ruff` on the path | (the tool check fails first: `missing tool: ruff (<hint>)`) | FAILED |
| off its pin | `ruff --version` is not `ruff <pin>`, or the pin file has no `ruff==` line | `ruff VOID: <reason>`; no lint or format call made | FAILED |
| refused | the listing, `ruff check` or `ruff format --check` exits 2 | `ruff VOID: <reason>` | FAILED |
| empty | the listing names no file, or the format summary names no count | `ruff VOID: <reason>` | FAILED |
| two populations | the format count differs from the listing's | `ruff VOID: <reason>` | FAILED |
| red | `ruff check` or `ruff format --check` exits 1 | `ruff check examined <n> file(s), exit <c>; ruff format examined <n> file(s), exit <c>` | FAILED |
| clean | both exit 0, one count | the same two fields, each exit 0 | ok, if every suite is |

Each listing, lint and format call passes `--no-cache` and `RUFF_ROOTS` (`scripts tools deploy agent crates`), so the
stage writes no cache into the checkout that `scrub` and `secrets` read after it.

The computed-root guard is a module of `scripts/tests`, which the stage's first suite discovers:

```mermaid
flowchart TD
  walk["every .py file under scripts, tools, deploy, agent and crates"] --> fenced{"a name FENCED lists?"}
  fenced -->|"yes"| skipped["skipped before any open, counted as skipped"]
  fenced -->|"no"| parse{"ast.parse"}
  parse -->|"fails"| unparsed["refused by name"]
  parse -->|"parses"| sites["each assignment to a ROOT or REPO name from Path(__file__)"]
  sites --> shape{"resolve() optional, then parents[N] or k times parent?"}
  shape -->|"no"| unresolved["refused: a chain the guard cannot resolve"]
  shape -->|"yes"| depth{"the repository root, or the directory LISTED_ROOTS names?"}
  depth -->|"no"| wrong["refused by path, line, name and the directory it reaches"]
  depth -->|"yes"| counted["counted"]
  counted --> examined["examined files and sites printed; zero of either fails"]
```

At ac7ebd7 the guard counts eleven sites, all at the repository root, and `LISTED_ROOTS` is empty.
