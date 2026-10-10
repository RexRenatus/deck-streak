### Added

- `scripts/dynamic-imports-check.py` refuses, before the push, a new or changed test module under
  `scripts/tests` that loads code through `importlib`, `runpy` or `exec` while the
  `DYNAMIC_IMPORTS` register in `scripts/tests/test_ci_workflows.py` names no site of it. The SPEC
  template's file manifest section now tells an author to list that register when a delivery adds
  such a load.
