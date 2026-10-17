# Red-first record: SPEC-473

The red commit adds the Lean entry `lean/DeployEntryRefusal`, its vectors writer and its vectors,
and the two test changes, against `deploy.sh` as it stands: there the settings check is top-level
code of the Caddy arm, and no function holds it. The next commit moves the check's lines into the
function `refuse_unlisted_settings` and edits no test file.

- A1, the formal checker's check of the entry, is red at the red commit: the theorem is clean and
  its witness built, and the cover reads `STALE` because its anchor, `refuse_unlisted_settings`,
  does not resolve. It is green once the cover is stamped and the vectors are written again. CI
  has no formal job, so both lines are the checker's, run on the committed tree.
- A2 and A3 run in CI only, because `test_deploy_scripts.py` executes the deploy scripts. Their red
  lines are quoted from CI's log of the job that runs that module at the red commit: A2 fails by its
  last assertion, the function absent, after every vector has run against the step; A3 fails where
  the step's recorded commands hold the refusal's first command in place of the declared call.

The fence lines follow in the record commit.
