# Red-first record: SPEC-353

SPEC-353, ADR-364 and the schematic were committed first (3f5c0f54). A1's, A2's and A3's tests,
with the rewritten helper of A7 to A9 and A38's stale names, were then committed alone with the
cure absent (0d826206), and read red by assertion in CI's python job, run 37303034052, job
111740058445. No builder ran the module (ruling 189), so that job and its run are the measurement.
The red commit edits only the test file. A7 to A9 and A38 stayed green at 0d826206 with their
rewritten helper and stales. The cure commit (994aee75) edits `deploy/deploy.sh` and
`deploy/README.md` and no test file.

```red-first
A1: red at 0d826206: Lists differ: [PosixPath('.../host/etc/cfdir/deck-streak.caddy')] != [PosixPath('.../host/etc/caddy/deck-streak.caddy')]
A2: red at 0d826206: PosixPath('.../host/etc/caddy/extra.caddy') != PosixPath('.../host/etc/cfdir/extra.caddy') : .../host/etc/caddy/deck-streak.candidate: the check resolves the operator's import elsewhere
A3: red at 0d826206: 'DECKSTREAK_DEPLOY_CADDY_DIR is not an absolute path of plain characters' not found in '' : the install names the setting
A1: green at 994aee75
A2: green at 994aee75
A3: green at 994aee75
```

One commit sits between the red and the cure: 6bf75cc1 re-implements the test helper `caddy_import` without `os.path`, changing no assertion, because the stand-in census counted its two `os.path` sites as unreviewed (ruling 356).

994aee75 was not pushed alone, so CI never ran it; its `deploy/deploy.sh` and `scripts/tests/test_deploy_scripts.py` blobs (22379a2d, e5b22a25) are the head's, and CI's python job read the module green at 0b313178 (run 37307839879, job 111756115842).

A3's criterion changed after the first fence above: it now also refuses a letter outside ASCII, run under `LC_ALL=en_US.UTF-8` after the test asserts that locale is installed. That member is committed alone before the cure that lists the guard's characters, and a changed criterion takes no second fence line. At the range spelling under `en_US.UTF-8` the guard admits a fullwidth letter, so the two new subtests (the install and the removal) fail by assertion: the refusal text is absent and the step goes on to need the host. Listing the characters refuses it in every locale (S35308 restores the range spelling and is killed by that member).
