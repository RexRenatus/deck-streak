# Red-first record: SPEC-043

A1 to A3 were run red against an inert runner stub and replayed from a checkout of that commit.
A4 and A5 were committed with their subjects (the manifest, prompts and red-team cases) in one
commit, so they are disclosed as not red. A6 to A14 were run red against inert stubs of the new
modules (an always-passing gate, a runner that always fails, an empty composer, a repository that
records nothing) and green against the real ones.

```red-first
A1: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 'CLAUDE_CODE_OAUTH_TOKEN=synthetic-device-key-0f3a9c' not found in '' : the key reaches claude in its environment
A1: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A2: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 0 != 2
A2: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A3: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 0 != 4
A3: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A4: not red: the settings template and the scan's test were committed together with the manifest, and the scan finds no helper in either
A5: not red: the manifest, prompts and red-team cases were committed with the structure test, and the real probes run only on the box (SPEC-043 section 3a)
A6: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: cards-exfil-link.md: AiRouteAbsent
A6: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A7: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A7: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A8: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A8: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A9: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: expected withheld, got AiRouteAbsent
A9: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A10: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: only the template's two fences open
A10: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A11: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: the_prompt_is_composed_in_the_persona_order panicked at crates/agent/tests/compose.rs:24:46
A11: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A12: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A12: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A13: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: 0
A13: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A14: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: []
A14: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A15: not red: the runner already read only the credentials directory, so the tests were written against a green runner; each is disclosed with its plant, rows S04318 (an environment fallback), S04319 (another credential path) and S04320 (a token path in a committed file), each proved killed
```

## Round 5 addendum

Round 5 added `test_the_runner_accepts_only_an_exact_loopback_url_and_an_absolute_path` (a sibling of the A2 test) and the runner's
exact-URL and absolute-path checks. It was committed alone and run red against the previous
`agent/run-headless.sh`: five of its six refusal cases failed by assertion (the sixth, a remote
`http://` URL, was already refused), each with `AssertionError: 0 != 2`, and the four loopback forms
launched. The A1 to A3 lines above stand, and each id is recorded once, so the round-5 pair is
quoted here and not repeated in the block above.

```text
red at a6f20dc911efbcf5860541860c1cbaede5657c2e: AssertionError: 0 != 2 : (five refusal subtests)
green at d78d60cd8bd7076f09a493128349e3d7f96485ae: the agent tests pass, with the four loopback forms launching
```

## Amendment addendum, 2026-09-29 (issues #362 and #363)

A16 to A20 were committed alone, against inert stubs where a symbol was missing: `ProbeGate::new`
took the new arguments and returned a `Result` but refused nothing, migration 004302 was absent,
and for A18 the `#[must_use]` on `Verdict` was removed in the red commit as a plant and restored in
the green commit, so A18 was red by that plant and not by any earlier state of the tree. The lines
above stand.

```red-first
A16: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: None right: Some(NoOutputClass) (gate.rs:157)
A16: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A17: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: None right: Some(NoInputClass) (gate.rs:181)
A17: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A18: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: `pub enum Verdict` lost its #[must_use] (verdict.rs:32; red by the disclosed plant)
A18: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A19: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: [] (runs.rs:30)
A19: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A20: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: the prune does not use the index (runs.rs:51)
A20: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
```
