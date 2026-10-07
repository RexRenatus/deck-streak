### Changed

- The mutation gate's per-mutant timeout is 2300 seconds, by the owner's signed ruling in
  `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2300.md`, at the ten places that hold it
  equal. The sizer's census term is 1521 s and its baseline 2141 s, from one measured run, so a
  release's progression legs are judged instead of reading VOID. `--build-timeout`, each leg's
  six-hour job limit and the shard bound are unchanged, and no mutant is skipped or deferred (SPEC-362,
  ADR-373; refs #692).
