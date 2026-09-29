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

Fix round 1: A13 to A17 were written whole at 40e68bb, before any change to the scripts, and each
failed by assertion there over the whole test module; their greens are at 52e5aa5. A12 and A18
execute behaviour that was already right (the readiness gate and the tag guard), so neither has a
red line; their rows S06216 and S06215 show each fails when its behaviour is removed.

Fix round 2: A19 and A20 read behaviour that was already right (the Caddy block's source and the
token's scope), so neither has a red line; their rows S06223 and S06224 show each fails when its
behaviour is removed, and S06225 shows A15 fails when the deploy stops naming the unit it cannot
show. The merge with dev brought SPEC-066's unit guards, and two of them were red on the merged tree
for the sync instance's drop-in (the drop-in directory guard and the credential-bearing unit set);
A21 is the third test, beside them, and the guards now read an instance's drop-ins. After the green
commit of round 1, two test files were changed, as a later commit in the same round.

Fix round 3: A21 now plants a key in the instance's drop-in and asserts that the unit guards refuse
it, because the reader reads an instance's drop-ins with its template. Its red line is the test run
against the reader as it stood, and its green commit is the one that changes the reader.

Fix round 4: A21 also plants a second instance directory of the shipped template and asserts that
both are refused, because the reader merges every instance directory of a template into one unit
and a second would let one instance's setting mask another's. The record admits one red and one
green line per criterion, so this round's pair is disclosed here. A21 failed by assertion at
6b241af over the whole module (21 tests, one failure): `AssertionError: Lists differ` between the
refusal list and the list holding the two `planted@...` lines, `a second instance of the shipped
template`. Its green is 062cc5a. The template's own drop-in directory was read twice by the
instance glob; a new test, `test_a_templates_own_dropin_directory_is_read_once`, failed by
assertion at 1dbbc73 (two identical refusal lines where one was expected) and is green at c967a20,
with the row S06227 pinning the glob and S06226 the count.

Fix round 5: A22 plants a template's setting restated in its one instance's drop-in and asserts
that the unit guards refuse it, because systemd applies an instance's drop-in to that instance
alone while the guards read it with the template. It failed by assertion at 4777c36 over the whole
module (23 tests, one failure) and is green at f369943, with the row S06228 pinning the key rule.

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
A12: not red: the guard is right at the dispatch head and the test only executes it; the row S06216 proves it fails when the readiness gate is removed
A13: red at 40e68bb: AssertionError: 'v1.0.0' not found in ['v1.0.1', 'v1.1.0', 'v1.2.0'] : the release it replaced was pruned
A13: green at 52e5aa5
A14: red at 40e68bb: AssertionError: False is not true : getty@tty1.service.d/autologin.conf was deleted by a deploy
A14: green at 52e5aa5
A15: red at 40e68bb: AssertionError: 0 == 0 : a deploy whose effective view was partial
A15: green at 52e5aa5
A16: red at 40e68bb: AssertionError: Lists differ: ['/etc/systemd/system', '/run/systemd/system', '/etc/systemd/sy[47 chars]ser'] != ['/etc/systemd/system']
A16: green at 52e5aa5
A17: red at 40e68bb: AssertionError: 1 != 0 : invalid character: not JSON
A17: green at 52e5aa5
A18: not red: the workflow's guard is right and the test only executes it; the row S06215 proves it fails when the guard is weakened
A19: not red: the Caddy install already renders from the tag; the row S06223 proves the test fails when it renders from the working tree
A20: not red: the token is already scoped to the three release steps; the row S06224 proves the test fails when it is set at job level
A21: red at 6938671: AssertionError: Lists differ: [] != ["deploy/systemd/planted@tty1.service.d/10[96 chars]sed"]
A21: green at 59d8186
A22: red at 4777c36: AssertionError: Lists differ: [] != ["deploy/systemd/planted@tty1.service.d/10[131 chars]sed"]
A22: green at f369943
```
