# Red-first record: SPEC-353

SPEC-353, ADR-364 and the schematic were committed first (3f5c0f54). A1's, A2's and A3's tests,
with the rewritten helper of A7 to A9 and A38's stale names, were then committed alone with the
cure absent (0d826206), and read red by assertion in CI's python job, run 37303034052, job
111740058445. No builder ran the module (ruling 189), so that job and its run are the measurement.
The red commit edits only the test file. A7 to A9 and A38 stayed green at 0d826206 with their
rewritten helper and stales. The cure commit (994aee75) edits `deploy/deploy.sh` and
`deploy/README.md` and no test file.

```red-first
A1: red at 0d826206: Lists differ: [.../host/etc/cfdir/deck-streak.caddy] != [.../host/etc/caddy/deck-streak.caddy]
A2: red at 0d826206: PosixPath('.../host/etc/caddy/extra.caddy') != PosixPath('.../host/etc/cfdir/extra.caddy') : the check resolves the operator's import elsewhere
A3: red at 0d826206: 'DECKSTREAK_DEPLOY_CADDY_DIR is not an absolute path of plain characters' not found in '' : the install names the setting
A1: green at 994aee75
A2: green at 994aee75
A3: green at 994aee75
```

One commit sits between the red and the cure: 6bf75cc1 re-implements the test helper `caddy_import` without `os.path`, changing no assertion, because the stand-in census counted its two `os.path` sites as unreviewed (ruling 356).
