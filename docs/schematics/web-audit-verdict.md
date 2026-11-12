# Schematic: the web audit, from pnpm's report to the stage's verdict

Kind: data flow and state machine. Read at DeckStreak `dev` 16ed8e2 (`scripts/check.sh`,
`pnpm-workspace.yaml`) and pnpm 11.27.1's JSON audit report. Decided by ADR-055's note of
2026-09-28; built by SPEC-058.

## The data flow

```mermaid
flowchart LR
  lock[("pnpm-lock.yaml: every importer's dependencies,<br/>devDependencies and optionalDependencies")] --> pnpm
  level[["AUDIT_WEB_LEVEL=low, in check.sh"]] -->|"--audit-level low"| pnpm["pnpm audit --json --audit-level low<br/>(no flag that narrows the classes)"]
  pnpm <-->|"each package version, once"| registry[("the registry's advisory endpoint")]
  pnpm -->|"the JSON report, on stdout"| verdict["scripts/audit-web-verdict.py"]
  pnpm -->|"its exit code, --pnpm-exit"| verdict
  level -->|"--level low"| verdict
  verdict -->|"one line per failing advisory,<br/>then the verdict line and its exit"| stage["the audit-web stage's log, and<br/>check.sh's summary line"]
```

The level is defined once and reaches both readers, so the report pnpm filters and the verdict that
judges it hold the same bar. A workspace's `audit.level` applies only when the command line names
none, so it cannot raise the bar the gate states. The count the verdict prints is
`metadata.totalDependencies`, beside the report's own `dependencies`, `devDependencies` and
`optionalDependencies`.

## The verdict

```mermaid
stateDiagram-v2
  [*] --> read: the report on stdin, and pnpm's exit
  read --> void: not a JSON object, or no whole-number totalDependencies, or no advisories object
  read --> counted: totalDependencies is N
  counted --> void: N is 0
  counted --> judged: N is 1 or more
  judged --> failed: an advisory at or above the level, or of none of pnpm's grades
  judged --> failed: no such advisory, and pnpm exited non-zero
  judged --> ok: no such advisory, and pnpm exited 0
  void --> [*]: "audit-web: VOID: ...", exit 3
  failed --> [*]: each advisory named, then the verdict line, exit 1
  ok --> [*]: the verdict line, exit 0
```

pnpm's grades, lowest first, are `info`, `low`, `moderate`, `high` and `critical`; `--audit-level`
takes the last four. An advisory fails when its grade is at or above the level, and one whose
severity is none of the five fails too, because a grade the verdict cannot place is never read as
clean. The report decides: an advisory it holds fails the stage whatever pnpm's exit was, and a
pnpm that exited non-zero is never a pass.
