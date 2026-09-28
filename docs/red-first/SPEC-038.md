# Red-first record: SPEC-038

The tests were committed (fa9dcfd) before the implementation, with `scripts/pack-rows.py` given a
`--jobs` stub that accepted a bound and still ran one row at a time, so A9 fails by assertion and
not by a usage error. Every sha below was re-run from a `git archive` export. Three criteria pin a
property the single gate job already had, so they are disclosed as not red: they guard what the
split must keep, and each refuses a planted defect.

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
```
