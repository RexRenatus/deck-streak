# Red-first record: SPEC-344

The SPEC, its schematic and ADR-355 were committed first (434f3e95). Each criterion's test was then
committed alone, and the first push carried those commits and the fragment only. The reds below
are quoted from the `hygiene` job's python stage log of run 37236066842 at head 58d03e66, where
the suite read `Ran 947 tests` and `FAILED (failures=9)`. The change is e538a73a.

```red-first
A1: red at 58d03e66: AssertionError: 'apple-on-change.yml' not found in ['changelog.yml', 'ci.yml', 'engine-measure.yml', 'mutation-weekly.yml', 'release.yml', 'rust-cache.yml', 'xcframework.yml']
A1: green at e538a73a
A2: red at 58d03e66: AssertionError: False is not true : .github/workflows/apple-on-tag.yml does not exist
A2: green at e538a73a
A3: red at 58d03e66: AssertionError: Lists differ: ['pull_request', 'workflow_dispatch'] != ['workflow_call', 'workflow_dispatch']
A3: green at e538a73a
A4: red at 58d03e66: AssertionError: Lists differ: [] != [('apple-on-change.yml', 'apple', './.gith[96 chars]ml')]
A4: green at e538a73a
A5: red at 58d03e66: AssertionError: 'reacher' != 'release'
A5: green at e538a73a
A6: not red: the callee resolves only the locked graph at the base, a guard that stays green
```

- The pin-test amendment (A4) is mutation coverage, not a red-first criterion of its own: every
  refusal it had stays refused, and its planted controls hold that.
- The first run also read the file-read census red in `WorkflowFilesAreReadAsBytes` (its census
  test and three planted-site controls). That is not a criterion's red, and its cause is not
  settled here; the completing head's run records it by name.
