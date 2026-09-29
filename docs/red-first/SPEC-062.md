# Red-first record: SPEC-062

The order of work: the SPEC promoted, its R14 and A11 added and ADR-062 accepted (3510265); every
criterion's test written whole beside no implementation (300eca2); the sync login moved to the sync
instance's drop-in (d55ecd2) and ADR-061 amended (c9acf01); the release workflow (4aa9760, its
tarball layout fixed at 19884de); the Caddy render (e661e60); the deploy and the rollback
(bc0665a); the mutation rows (997caac); and one assertion added to A8 after a row survived
(bd70d53).

Each criterion was run from a `git archive` export of 300eca2, with the SPEC's own fenced command,
selecting one test. A1 to A6 and A10 failed on an assertion that the script, `deploy/deploy.sh`,
did not exist; A7 and A8 and A9 on the same assertion for `render-caddy.py` and `release.yml`; A11
on the assertion that the job template still requested a sync credential. None failed by a
compile error, a missing fixture or an empty selection.

A8's assertion that the workflow proves the tagged commit itself is on `main` was added at bd70d53,
after the row S06215 in which that comparison was replaced by a self-comparison survived the test
as first written. The row is killed with it.

```red-first
A1: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A1: green at bc0665a
A2: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A2: green at bc0665a
A3: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A3: green at bc0665a
A4: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A4: green at bc0665a
A5: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A5: green at bc0665a
A6: red at 300eca2: AssertionError: deploy/deploy.sh does not exist
A6: green at bc0665a
A7: red at 300eca2: AssertionError: deploy/scripts/render-caddy.py does not exist
A7: green at e661e60
A8: red at 300eca2: AssertionError: .github/workflows/release.yml does not exist
A8: green at 4aa9760
A9: red at 300eca2: AssertionError: .github/workflows/release.yml does not exist
A9: green at 4aa9760
A10: red at 300eca2: AssertionError: False is not true : deploy/deploy.sh does not exist
A10: green at bc0665a
A11: red at 300eca2: AssertionError: 'anki-sync-username' unexpectedly found: the template requests a sync credential
A11: green at d55ecd2
```
