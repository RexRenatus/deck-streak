# Red-first record: SPEC-656

SPEC-656 (R1 to R8, A1 to A5). The census module with its tests, its fixtures and stubs that keep
every input but the missing behaviour (the reader finds no site, the boundary is the API module
alone, the verdict prints no WARN and fails no unlisted site but keeps each STALE, and the population is never refused) was committed first
and alone, with `LISTED` and `LEVEL` already in place, so each red is its criterion's assertion
and never a missing name. A2 to A5 read planted text and were read locally by name; A1 reads the
repository and was read in CI's `hygiene` job.

```red-first
A1: red at 24c5963f: FAILED, failures holds 23 STALE absent-spelling lines
A1: green at 756b519a
A2: red at 24c5963f: assertion failed, left: [] (no planted loose comparison found)
A2: green at 756b519a
A3: red at 24c5963f: assertion failed, left: [] (no planted boundary site found)
A3: green at 756b519a
A4: red at 24c5963f: assertion failed, left: ([], ['STALE loose-equality web/app/src/b.ts #1']) (no WARN line)
A4: green at 756b519a
A5: red at 24c5963f: AssertionError not raised (an empty population not refused)
A5: green at 756b519a
```

The reader's five bodies and two import lines replaced the stubs in 756b519a, with no test
function, planted text or expected value changed; the commit after it changes only this record.

The red texts above name each failure in words. At the red commit the local runs printed these
first failure lines, as unittest prints them (a long value is cut by its own `[N chars]` form):

- A2 printed `AssertionError: Lists differ: [] != [('web/app/src/lib/api.ts', 21), ('web/app[101 chars] 15)]`
- A3 printed `AssertionError: Lists differ: [] != [('web/app/src/lib/Header.svelte', 10), ('[201 chars], 6)]`
- A4 printed `AssertionError: Tuples differ: ([], ['STALE loose-equality web/app/src/b.ts #1']) != (['WARN loose-equality web/app/src/a.ts #1 [50 chars]#1'])`
- A5 printed `AssertionError: AssertionError not raised`

A1 reads the repository, so its red is the first push's `hygiene` failure, read there by its
FAIL header.
