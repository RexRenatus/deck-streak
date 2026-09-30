# Red-first record: SPEC-196

The SPEC, in `docs/specs/planned/`, and ADR-199 were committed alone (b83f753a). A probe of the
runner's memory scope followed (3b10e34e) and was removed (75665275). Then came the tests of A1
to A17 (95907c06) against the unchanged scripts and workflows, so every test ran and each red
failed by assertion. The script (607fd3f5) turned A1 to A6 green, the verdict (c30e673b) turned A9
to A16 green, and the workflows (e81e1c05) turned A7 green. The replay ran both test files on
95907c06's tree: seventeen criteria, fourteen red by assertion.

```red-first
A1: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A1: green at 607fd3f5
A2: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A2: green at 607fd3f5
A3: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A3: green at 607fd3f5
A4: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A4: green at 607fd3f5
A5: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A5: green at 607fd3f5
A6: red at 95907c06: 'scripts/memory_scope.py is absent: the memory scope is not built'
A6: green at 607fd3f5
A7: red at 95907c06: Lists differ: ['ci.yml:413: a run that runs tests is not[162 chars]ope'] != []
A7: green at e81e1c05
A8: not red: the arguments of the three wrapped commands and the four listing commands were already the pinned ones; the criterion pins that they stay so
A9: red at 95907c06: 0 != 3 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A9: green at c30e673b
A10: not red: a shard the cap never touched was already judged as the criterion says; the criterion pins that it stays so
A11: red at 95907c06: Lists differ: [] != ['mutation-rust-shard-0: MEMORY-CAP crates[110 chars]out']
A11: green at c30e673b
A12: red at 95907c06: 0 != 1 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A12: green at c30e673b
A13: red at 95907c06: 0 != 1 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A13: green at c30e673b
A14: red at 95907c06: 'battery: mutants-shard-0: MEMORY-CAP crates/fix/src/lib.rs:3:5: replace double -> i64 with 3: the memory cap stopped its tests; neither caught nor a timeout' not found in ['battery: MISSING rows: no rows.json', 'battery: MISSING stryker: no mutation.json', 'battery: counted 3 of 5 reports whole', 'examined 5 report(s)']
A14: green at c30e673b
A15: red at 95907c06: 'REPLACED crates/fix/src/lib.rs:3:5: replace double -> i64 with 3' not found in 'mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)\nmutation: rust: cargo-mutants examined 51 (caught 51, missed 0, timeout 0), unviable 0, of 51 on the diff\nmutation: rust: missed 0: equivalent 0, unexplained 0\nmutation: rust: examined 51 by cargo-mutants and 0 by rows\nmutation: rust: verdict: ok\nexamined 51\n'
A15: green at c30e673b
A16: red at 95907c06: 0 != 1 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A16: green at c30e673b
A17: not red: the rehearsal's command, its file and its examined sum were already the ones before this SPEC; the criterion pins that they stay so
```

## Measured on the runner

- Probe (3b10e34e, run 36634258307): the scope was in force; cargo-mutants examined 6 without it (n0 = 6).
- Green state (e258e28c, run 36639697316): the rehearsal ran inside the scope, the kernel stopped 0
  processes at the cap, and `rehearsal: cargo-mutants examined 6` (n1 = 6, equal to n0).
- Plant (131c2bdd, run 36640720516): the plant gave nine mutants, and exactly one never ended,
  `replace > with >= in plant_chunks`. The leg's scope record read `in_force` true with one
  kernel stop, and the leg's log read `stopped 1 process(es) at the cap`. The verdict held one
  failure, `mutation-rust-shard-0: MEMORY-CAP crates/kernel/src/memory_cap_plant.rs:11:13: replace > with >= in plant_chunks: the memory cap stopped its tests; neither caught nor a timeout`,
  no VOID, and `examined 8 by cargo-mutants` (nine less the one). Only `mutation-verdict` and the `ci`
  aggregate were red at that sha; the plant was removed by the next commit.
- Fixture change: the real status line and its summary repeat both carry the `(n/m)` counter
  (`SIGKILL [ ...s] (58/58) <binary> <test>`). The A12 and A16 summary fixture had no counter, so
  it follows the real line (63428d94). The verdict's reading accepted both shapes, so no verdict
  code changed.
- Expectation change: the verdict's commit (c30e673b) also corrected one A12 sub-case, `no placed
  kill`, whose expected counts read `(0, 1, 0)` for a record of one out-of-memory event and one
  kill; they read `(1, 1, 0)`, the record's own counts. A12 was red at 95907c06 by the assertion
  quoted above, before that sub-case is reached.
- Fixture change: cargo-mutants opens every scenario log with a blank line, then
  `*** <scenario>`. The fixture's logs opened with the `***` line, so a shard whose report is not
  whole named its stopped mutant by the log's file name on the real shape. The fixture follows the
  real opening, the verdict reads the first line that is not blank, and row S19630 holds the arm.
