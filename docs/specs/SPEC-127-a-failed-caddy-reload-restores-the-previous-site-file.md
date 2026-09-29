# SPEC-127: a failed Caddy reload restores the previous site file

- **Wave:** W2. **Issue:** #321. **Context(s):** `repo` (`deploy/deploy.sh`).
- **Decided by:** ADR-127, which keeps the previous copies until the reload succeeds and restores them
  when it does not.
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-127.md` (ADR-016).

## 1. The problem, measured

In `deploy.sh`'s `caddy_install` host script, once `caddy validate` and `caddy adapt --validate`
pass on the candidate copy, the candidate is moved over the Caddyfile and the previous block
(`deck-streak.caddy.previous`) is deleted, and only then does `caddy reload` run. Read at `dev`
dd478d7. A reload that fails leaves the new block and the new Caddyfile on disk, Caddy running the
old configuration, and no previous copy to restore: the files and the running configuration differ
until some later reload succeeds. `caddy_remove` has the same order and the same gap: it moves the
candidate over the Caddyfile and deletes the block before its reload. The judgment: the gap is the
same, the issue's intent (files that match what Caddy runs) covers both, and so R4 includes it.
Caddy's own documentation says that on a reload with errors it "automatically reverts to the last
known working configuration" (Caddy, Getting started: Reloading config), which is why the running
side is safe and only the disk side is wrong.

## 2. Requirements

R1. `caddy-install` keeps the previous block and the previous Caddyfile until the reload succeeds.
R2. A failed reload restores both, reloads the restored configuration, and exits non-zero with a
    message that names the failed reload. If the restoring reload also fails, a second message says
    so distinctly, and the exit is still non-zero.
R3. A first install (no previous block) whose reload fails removes the new block and restores the
    previous Caddyfile.
R4. `caddy-remove` follows the same rule: the block and the Caddyfile are kept until its reload
    succeeds, and a failed reload restores both, as R2 says.
R5. The previous copies are removed only after a successful reload, and none is left behind after a
    success or a restored failure.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a failed reload restores the previous block and Caddyfile, reloads them, refuses and names the reload | `test_deploy_scripts.py` `a_failed_reload_restores_the_previous_block_and_caddyfile` |
| A2 | a failed restoring reload is named distinctly and the install still refuses | `test_deploy_scripts.py` `a_failed_restoring_reload_is_named_apart_and_still_refuses` |
| A3 | a first install whose reload fails removes the new block and restores the Caddyfile | `test_deploy_scripts.py` `a_first_install_whose_reload_fails_removes_the_new_block` |
| A4 | the previous copies exist at the reload and are gone after a good one | `test_deploy_scripts.py` `the_previous_copies_outlive_the_reload_and_go_after_a_good_one` |
| A5 | a failed reload of `caddy-remove` restores the block and the Caddyfile, and a good one removes them | `test_deploy_scripts.py` `a_failed_reload_of_the_removal_restores_the_block_and_caddyfile` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_reload_restores_the_previous_block_and_caddyfile
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_restoring_reload_is_named_apart_and_still_refuses
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_whose_reload_fails_removes_the_new_block
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k the_previous_copies_outlive_the_reload_and_go_after_a_good_one
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_reload_of_the_removal_restores_the_block_and_caddyfile
```

The tests use the existing fakes: a `caddy` stub whose `reload` fails on demand (the count of
failures is written to a file, so one file fails the reload once and another twice) and records how
many previous copies sit beside the configuration at each reload. No test reaches a host.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/deploy.sh` | repo | changed: R1 to R5 (`caddy_install`, `caddy_remove`) |
| `scripts/tests/test_deploy_scripts.py` | repo | changed: the `caddy` stub and A1 to A5 |
| `scripts/mutation-rows.d/S12700-S12799.json` | repo | added: the restore clauses as script rows (section 7) |
| `deploy/README.md` | repo | changed: one sentence in the Caddy section |
| `docs/specs/SPEC-127-a-failed-caddy-reload-restores-the-previous-site-file.md` | repo | added |
| `docs/decisions/ADR-127-a-caddy-reload-that-fails-restores-the-copies-kept-until-it-succeeds.md` | repo | added |
| `docs/decisions/ADR-062-a-deploy-installs-only-a-release-whose-provenance-and-digests-verify.md` | repo | changed: the unit-guards bullet replaced (the advisory); one bullet |
| `docs/specs/SPEC-062-first-deploy-units-caddy-block-and-https.md` | repo | changed: one dated amendment line at its end (insert-only) |
| `docs/red-first/SPEC-127.md` | repo | added |
| `changelog.d/fix-caddy-restore-127.md` | repo | added |

## 5. What this does NOT do

- It does not run `caddy reload` on a candidate path before the swap. The block must sit in place
  for the import to read it, so the candidate order saves no restore (#321).
- It does not read the health-check list after a restore; the deploy's readiness probes stay as
  SPEC-062 gives them (#321).
- It does not restore anything when the host is unreachable or the script is killed between the
  swap and the reload; the next `caddy-install` starts from the files on disk (#321).
- It does not change the validation of the candidate copy, the rendering of the block or the
  release install path (#321).
- It performs no step of the cutover (#164).

## 6. Risks

- **The restoring reload fails too.** The files are already back and the exit is non-zero with a
  message that says so, so the operator reads the state from the message (R2, A2).
- **The Caddyfile's previous copy is left after a crash.** It is named beside the Caddyfile, so the
  next run overwrites it before use and it is never read as current (R1).

## 7. Mutation rows

`scripts/mutation-rows.d/S12700-S12799.json` holds script rows on `deploy/deploy.sh`, each proved
killed by its full id and each naming one test as its killer:

| row | the invariant it pins | killer |
|---|---|---|
| S12701 | the previous block is put back when the reload fails | A1 |
| S12702 | the previous Caddyfile is put back when the reload fails | A3 |
| S12703 | a failed reload exits non-zero | A2 |
| S12704 | the previous copies are not deleted before the reload | A4 |
| S12705 | the removal restores the Caddyfile when its reload fails | A5 |

## References

SPEC-062 (R7, whose exclusion this closes), ADR-062, ADR-127; #321.
