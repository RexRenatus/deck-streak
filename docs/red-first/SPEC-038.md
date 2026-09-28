# Red-first record: SPEC-038

The tests were committed (fa9dcfd) before the implementation, with `scripts/pack-rows.py` given a
`--jobs` stub that accepted a bound and still ran one row at a time, so A9 fails by assertion and
not by a usage error. Every sha below was re-run from a `git archive` export. Three criteria pin a
property the single gate job already had, so they are disclosed as not red: they guard what the
split must keep, and each refuses a planted defect.

A1 and A5 were amended after `dev` brought SPEC-042's rails test, whose python stage runs `cargo`.
The amended tests were committed first (c89191f) and were red there for their own reasons:
`AssertionError: [] is not true : hygiene installs no pinned toolchain` for A1, and, for A5,
`Regex didn't match: '^FAILED +python .*: missing tool: cargo \(\S.*\)$' not found in 'ok python 0s'`
(the python stage ran with no cargo on the path). Both were green at afa366e.

A14 and A15 came with the engine (#204, merged into `dev` as b1ce32d). Their tests were committed
before that merge (d50e90d), and the merge (34bf4fa) is their implementation: resolving its two
conflicts carried #204's `protoc` step into both jobs that compile Rust, and its `protoc` check
into the three stages that compile the engine.

```red-first
A1: red at fa9dcfd: AssertionError: 0 != 1 : gate restores no Rust cache before its stages
A1: green at 77107bf
A2: red at fa9dcfd: AssertionError: Lists differ: ['ci.yml:gate:actions/setup-node: setup-node saves a cache, because it has a cache input or package-manager-cache is not false'] != []
A2: green at 77107bf
A3: red at fa9dcfd: AssertionError: {'gate': ['toolchain', 'fmt', 'clippy', 'test', 'doctest', 'web', 'python', 'packs', 'scrub', 'audit', 'secrets']} != the owner's four jobs (one gate job ran every stage)
A3: green at 77107bf
A4: not red: the single gate job already ran every stage exactly once; the test guards the split, and refuses a planted workflow that drops a stage, runs one in two jobs, or names one check.sh lacks
A5: red at fa9dcfd: 15 of 15 cases failed, among them AssertionError: Regex didn't match: '^FAILED +fmt .*: missing tool: cargo \(\S.*\)$' not found in 'FAILED fmt 0s exit 127: scripts/check.sh: line 47: cargo: command not found' (only the toolchain stage named a missing tool, and the test stage read ok with a cargo stub and no cargo-nextest)
A5: green at 77107bf
A6: red at fa9dcfd: AssertionError: Lists differ: ['audit', 'clippy', 'doctest', 'fmt', 'packs', 'python', 'scrub', 'secrets', 'test', 'toolchain', 'web'] != ['audit-rust', 'audit-web', 'clippy', 'doctest', 'fmt', 'packs', 'python', 'scrub', 'secrets', 'test', 'web']
A6: green at 77107bf
A7: not red: the single gate job already checked out the whole history and set CHECK_HISTORY=1; the test pins that every job reading history still does after the split
A8: not red: the runner at the base ran one row at a time, so a run the stub gave --jobs 4 decided exactly what a serial run did; the test guards the pool's row order, verdicts, timeouts and exit on every verdict kind
A9: red at fa9dcfd: AssertionError: 1 != 3 : the most rows seen running at once: [1, 1, 1, 1, 1, 1]
A9: green at 77107bf
A10: red at fa9dcfd: AssertionError: Lists differ: ['two pushes to dev share a group, so a third would cancel the pending one', 'a newer run of a pull request leaves the superseded one running'] != []
A10: green at 77107bf
A11: red at fa9dcfd: AssertionError: examined 0 browser caches: the population is empty, so nothing was judged (no step cached the browser)
A11: green at 77107bf
A12: red at fa9dcfd: AssertionError: False is not true : no timings.tsv in the log directory
A12: green at 77107bf
A13: red at fa9dcfd: AssertionError: Lists differ: ['.check-logs'] != [] (the stage logs sat in a hidden directory the upload skips)
A13: green at 77107bf
A14: red at d50e90d: AssertionError: 0 != 1 : hygiene installs no checksum-verified protoc (no job installed the engine's build tool before #204 was merged in)
A14: green at 34bf4fa
A15: red at d50e90d: AssertionError: Regex didn't match: '^FAILED +clippy .*: missing tool: protoc \(\S.*\)$' not found in 'ok clippy 0s' (clippy, test and doctest each ran with no protoc on the path)
A15: green at 34bf4fa
```
