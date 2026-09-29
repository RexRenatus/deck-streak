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
