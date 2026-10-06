### Changed

- A release pull request's Rust mutants are judged in one run, as every pull request's are: each
  `mutation-rust` leg runs for up to six hours and is sized within half of that, and the legs never
  outnumber what the run can hold beside its other jobs. A range that needs more is refused whole,
  by name, and every plan prints `legs N of ceiling C` (SPEC-362, ADR-373).
- The per-mutant timeout is 2200 s, covering the re-measured progression census with its 1.5
  margin, by the owner's signed ruling; the baseline and five package prices are re-measured.
- The verdict holds the plan's legs to the tool's own listing, counts every planned leg, and reads
  a leg VOID when its slowest baseline test, with the margin, outgrows the per-mutant timeout.
- A mutant's tests stop at their first failure (nextest's `mutants` profile); a mutant no test
  kills still runs every test.
- The scheduled battery sizes the whole tree from its listing at its own ceiling, no longer at a
  fixed 32 legs. No mutant, leg or row is skipped, capped or deferred.
