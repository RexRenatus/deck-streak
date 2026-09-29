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
```
