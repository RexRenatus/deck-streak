### Fixed

- The workflow scan of `test_mutation_workflows.py` follows a standalone `--` only after the
  dispatch-shard guard's own wrapper form. It reads that form from the guard's text with `ast` and
  never imports the guard, so `cargo mutants` after `echo`, `env`, `timeout`, `git` or another
  script's `--` is refused, as the guard refuses it (#533). Two named limits stay, and each is
  pinned by a test: an option the guard would refuse, which the scan over-finds, and a `--` given as
  an option's value.
- The census of stand-ins that fall back to a real program reads the syntax tree. It parses every
  `*.py` file under `scripts/tests/` and each string constant one level deep, and resolves calls
  through import aliases. It reaches the handler, the `finally` block and every later statement of
  the enclosing blocks, with no line window and no `plant` word. The finder's copy check compares
  normalised bodies beside the name match, and the tree reads exactly its one ruled floor (#532).
  SPEC-129 takes an insert-only amendment against dev, decided by ADR-312, and mutation rows
  S12913 to S12919 pin the new arms.
