# Schematic: the CodSpeed bench lanes, from the plan to the upload and the record (SPEC-618, ADR-640)

Kind: data flow. Citations read at dev `de4c4d4d`.

Every run of `ci.yml` starts the four bench jobs. Each build job runs the plan, then builds with no
token; its upload job reads the plan's outputs, unpacks the archive and uploads by OIDC. The
aggregate `ci` needs all four (`.github/workflows/ci.yml:1162`, 17 jobs before this delivery and 21
with it). On a pull request the Rust build job also runs the record check over the records that pull
request adds or changes, so a refused record fails the build job and with it `ci`.

```mermaid
flowchart LR
  run["a pull request into dev or main, or a push to them"] --> plan["perf-plan.py --lane rust or web"]
  meta[("cargo metadata document")] --> plan
  diff["the changed paths, git diff HEAD^1 HEAD"] --> plan
  plan -->|"applicable, upload, reason"| build["bench-rust-build or bench-web-build: contents read"]
  build -->|"applicable true"| archive["tar archive, one-day artifact"]
  build -->|"needs outputs"| upload["bench-rust or bench-web: contents read, id-token write"]
  archive --> upload
  tool["bench-rust only: the pinned cargo-codspeed release binary, fallback none"] --> upload
  upload -->|"upload true"| action["CodSpeed action, mode simulation"]
  action --> check["CodSpeed Performance Analysis check, outside every ruleset"]
  record[("the records the pull request adds or changes under perf/regressions.d")] --> rec["perf-record.py check, a step of bench-rust-build on a pull request"]
  names[("perf/benchmarks.txt, held equal to the bench files")] --> rec
  rec -->|"a refusal fails the build job"| build
  build --> ci["the aggregate ci, which needs all four bench jobs"]
  upload --> ci
  check --> merge["the merge step reads the check and the record"]
  rec --> merge
```

## The outputs, as the plan writes them

| output | values | read by |
|---|---|---|
| `applicable` | `true` or `false` | the build job's measured steps |
| `upload` | `true` or `false`; `false` on a fork pull request | the upload job's every step |
| `reason` | the first changed path that reaches the lane, the examined count, the push line or the fork line | the upload job's first step and the step summary |

## Where each input comes from

- The changed paths: `--changed <file>`, or `git diff --name-only <base> <head>` in this
  repository, the `fetch-depth: 2` checkout `mutation-web-shard` uses (`.github/workflows/ci.yml:903`).
- The bench closure: the packages with a `bench` target in the `cargo metadata --format-version 1`
  document, and every path dependency they reach.
- The fork test: the head repository the event names, against this repository.
- The token: OIDC on the upload jobs alone; the workflow default stays `contents: read`
  (`.github/workflows/ci.yml:19-20`), as `release.yml` scopes its own grant
  (`.github/workflows/release.yml:36-39`).
- The records: `git diff --name-only --diff-filter=AM HEAD^1 HEAD -- perf/regressions.d`, copied
  into `$RUNNER_TEMP/records`, checked against the head ref `github.head_ref` passes through the
  step's environment.
- The benchmark list: `perf/benchmarks.txt`, one `<file stem>::<bench>` name per line.
- The Rust upload job's one tool: `cargo-codspeed`, installed by `taiki-e/install-action` at a
  pinned version with `fallback: none`; the web upload job installs nothing.
