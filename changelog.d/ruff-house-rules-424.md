### Added

- The gate's python stage lints every Python file under `scripts`, `tools`, `deploy`, `agent` and
  `crates` with one ruff, after its three suites. `scripts/ruff-requirements.txt` pins that ruff
  and hashes every wheel of its release, `ruff.toml` requires the same version and adds the settled
  rule set to ruff's default one, and the `hygiene` job installs it from the pin into a virtual
  environment before its gate step. A ruff that is missing, off its pin, refusing its
  configuration, listing no file or formatting a population other than the one it checked reads
  `ruff VOID` and fails the stage; a finding fails it with both arms' counts. Every site the set
  finds is cured without a change of behaviour or named in the per-file table, and the formatter's
  one change is made.
- A guard, `scripts/tests/test_computed_roots.py`, reads the syntax tree of every Python file under
  the same roots and proves that each repository root computed from `Path(__file__)` resolves to
  the repository root (SPEC-424, ADR-438).
