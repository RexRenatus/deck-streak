### Changed

- Each Rust mutation leg runs its own unmutated baseline before cargo-mutants, inside the memory scope and under a 3845 s budget sized from the largest whole baseline of the release run 39033655190 times SPEC-362's 1.5 margin, and cargo-mutants then skips its own, so a leg's tests are no longer cut by the 2300 s per-mutant timeout before any mutant is judged (run 39033655190 read 107 of its 192 legs that way).
- The mutation verdict reads a leg with no baseline record, a baseline that reached its budget and a failed one each as VOID by name, never a pass; every mutant keeps its timeout and none is skipped, and the verdict job prints each Rust package's Timeouts, naming every one that could be a false kill so that it is read by name.
