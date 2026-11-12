# SPEC-NNN: the title says what the change makes true

- **Wave:** W<n>. **Issue:** #N (and its epic #N). **Context(s):** `deck-streak-<context>`.
- **Decided by:** ADR-NNN (every SPEC names at least one ADR by id, or has one of its own number).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-NNN.md` (ADR-016).

## 1. The problem, measured

What is wrong or missing, with numbers, and the command or source that produced each number. For
a port: the v9 behaviour being ported (its module and function, never its private data), and what
the parity oracle will prove.

## 2. Requirements

R1. Each requirement is a sentence that is true or false of the finished work.
R2. Constants are stated exactly and name where they come from (the parity oracle's golden, an owner
    decision, a pack rule).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the behaviour, stated so a test can observe it | `cargo test -p deck-streak-<ctx> --test <target> -- --exact <name>` |

```acceptance
A1: cargo test -p deck-streak-<ctx> --test <target> -- --exact <name>
```

Every criterion has one line in the fence, and every line names a test the delivery writes first
(red, for the criterion's reason) and then makes pass. A criterion two commands decide takes two
lines. Commands in the fence are the shapes the tdd probe resolves: `cargo test -p ... --test ...
-- --exact ...`, `cargo nextest run ... -E 'test(name)'`, `python3 -m unittest discover -s DIR -p
FILE -k name`, `pnpm exec vitest run web/app/src/PATH -t "name"` (from the repository root, which
holds the Vitest projects config). The probe resolves no `pnpm --dir` form and no Playwright command:
a browser criterion names its Playwright spec in the table and its fence line runs a Vitest test.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/<ctx>/src/<file>.rs` | `deck-streak-<ctx>` | added |

Every file the delivery adds or changes. A file outside this list needs an amendment first.

A delivery that adds a module under `scripts/tests` that loads code through `importlib`, `runpy` or
`exec`, or adds such a load to a module there, lists `scripts/tests/test_ci_workflows.py` in this
table as changed: its `DYNAMIC_IMPORTS` register names each such site by its module, qualified name
and text, with its count, and the census in the python stage refuses a site the register does not
name. Before the push, `python3 scripts/dynamic-imports-check.py --base <the pull request's base>`
refuses a new or changed module there that loads code and that the register does not name.

## 5. What this does NOT do

- Each exclusion is one bullet and cites the issue that owns it, `#N`. An exclusion with no owner is
  a promise nobody holds.

## 6. Risks

- What could go wrong, and what would detect it (a test, a probe row, an SLO, the memory watch).
