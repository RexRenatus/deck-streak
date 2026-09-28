# Schematic: the CI jobs, and the caches each one reads and writes

Kind: component and data flow. Read at DeckStreak `dev` 8f91667 (`.github/workflows/ci.yml`,
`scripts/check.sh`, `scripts/pack-rows.py`). Decided by ADR-055; built by SPEC-038.

## The jobs

```mermaid
flowchart LR
  event{{a pull request into dev or main, or a push to dev or main}} --> rust
  event --> web
  event --> packs
  event --> hygiene
  event --> lint[workflow-lint: zizmor]
  event --> base[base-is-dev]
  subgraph gate[check.sh stages, each in exactly one job]
    rust[rust: fmt clippy test doctest audit-rust]
    web[web: web audit-web]
    packs[packs: packs, a bounded pool of pack rows]
    hygiene[hygiene: python scrub secrets]
  end
  rust --> ci{ci: every need succeeded}
  web --> ci
  packs --> ci
  hygiene --> ci
  lint --> ci
  base --> ci
  ci --> required([the required check ci])
```

The four gate jobs need nothing, so they start together; the run's wall time is the slowest of
them plus the aggregate. Each calls `bash scripts/check.sh <its stages>` once. `hygiene` and `packs`
check out the whole history (`fetch-depth: 0`): `secrets` scans every commit, `scrub` reads every
blob reachable from `HEAD`, and the sdd numbering row reads every branch. `rust` and `web` check out
one commit.

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
| Rust | `~/.cargo/registry/index/`, `~/.cargo/registry/cache/`, `~/.cargo/git/db/`, `target/` | `rust-<os>-<hash of rust-toolchain.toml>-<hash of Cargo.lock>`, falling back to the same toolchain | `rust` | `rust`, on a push that missed, after `cargo clean --workspace` |
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
