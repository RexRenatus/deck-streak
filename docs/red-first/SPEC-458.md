# Red-first record: SPEC-458

The SPEC, ADR-475, the schematic and the two amendments were committed first, and the red commit holds tests, one register entry
and this record, with every workflow untouched. At that commit the two enumerating tests and the colour test fail by assertion on
the workflows as they stand: the weekly `rust` leg holds no fetch, the rehearsal fetches after its baseline, the warm-up saves with
no fetch, and `mutation-rust` ends on no colour line. Each module reads a workflow file, so each red is read on CI by test name and
recorded when seen.

```red-first
A1: red at a714f500: AssertionError: Lists differ: [] != [2 elements], the first 'mutation-weekly.yml:rust: no step runs `cargo fetch --locked`'
A1: green at 300096bd
A2: red at a714f500: AssertionError: Lists differ: [] != ['rust-cache.yml:warm: no step runs `cargo fetch --locked`']
A2: green at 300096bd
A3: not red: it plants its own shapes into copies of a saving job, so it pins the judge and not the tree
A4: red at a714f500: AssertionError: Lists differ: [] != ['mutation-rust: the cargo-mutants step ends on no colour line']
A4: green at 300096bd
A5: not red: it plants its own shapes into copies of the Rust leg's step, so it pins the reader and not the tree
A6: not red: SPEC-332's and SPEC-416's tests and test_rust_cache_workflow.py pin what the base already did, and guard the widened judge
```

The three reds were read on CI at the red commit by test name: hygiene job 117344207445 of run 39014685424 failed exactly
`AllWorkflowsBaselineJobsFetchTheLockedGraph` (A1), `EveryRustCacheSaveFetchesTheLockedGraph` (A2) and
`TheMutantLegReadsRedWithNoWholeReport` (A4), 3 failures of 1290 tests, and nothing else in that job. The workflows commit
300096bd greens all three. The pushed head carries the rows, the changelog fragment and this record on top of it, and
`git diff --quiet 300096bd HEAD` over the three test modules and the three workflow files exits 0, so the tests and the
code they read are byte-identical between the green commit and the pushed head, whose CI reads the greens by name.

The rebuild onto current dev merged the original branch unchanged. There the leg runs its own
baseline first and cargo-mutants skips its own, so a red baseline never reaches the colour line. The
commit after the documents adds the tests of A13 and A14 with every workflow untouched, and a later
commit adds the step they read.

The step A13 reads and its three rows come in their own commit, which changes no test file.

```red-first
A13: red at 19ff4d75: AssertionError: Lists differ: [] != ["mutation-rust: the last step's `if:` is [160 chars]()`']
A13: green at 195157d8
A14: not red: it plants its own shapes into copies of the leg's colour step, so it pins the reader and not the tree
```

A13 read red by name in the pull request run 42056188572 at 19ff4d75, and 195157d8 greens it with
its test unchanged. The merge of current dev between them changes no line of the mutation-rust job
and no line of the module that holds A13, and the commits after 195157d8 change only documents.

The paragraph on the three reds says the workflows commit that greens them and the pushed head are
byte-identical over the three test modules and the three workflow files; that held only at the
original head. At this head the two merges with dev and the commits on top change all six files, so
A1, A2 and A4 are read green by name in the run at this pull request's head, as A13 is.
