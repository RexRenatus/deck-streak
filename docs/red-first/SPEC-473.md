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

One commit changes `scripts/tests/test_deploy_scripts.py` between A2's and A3's red at the red
commit and their green: 96bdbf0f, after the stamp. It moves A2's start onto the module's existing
route, `EXEC_ENTRIES`, and A2's elevation setting onto the one the tests' world already gives
every step; A3 and every assertion of A2 stay as they were. That route drops an empty entry, so
from that commit on A2 examines 60 of the 63 vectors, and the 3 whose block holds an empty entry
are named in the test and not run.

The green commit is the one that writes the vectors again,
b28d20d8bd3412d0e462ae85f5ab82ecb7cb4bb1. Nothing after it changes the deploy scripts, the tests
or the entry: `git diff --quiet b28d20d8bd3412d0e462ae85f5ab82ecb7cb4bb1 HEAD -- deploy scripts/tests formal`
exits 0, so the head CI runs holds the same script, tests and entry. A1's lines are the checker's,
run on the committed tree. Each of A2's and A3's red lines joins two lines of the stage log CI kept
for the job that runs `test_deploy_scripts.py` at the red commit, the assertion's first line and
the line where its message ends; `[...]` stands for the lines between.

```red-first
A1: red at 07af9a78a76533fe7d7515ba4d980ddcacd631f8: FORMAL REPORT STALE lean/DeployEntryRefusal/a_caddy_step_proceeds_exactly_on_listed_settings_each_given_once_with_a_value — deploy/deploy.sh::refuse_unlisted_settings does not resolve
A2: red at 07af9a78a76533fe7d7515ba4d980ddcacd631f8: AssertionError: Tuples differ: (0, 0) != (1, 1) [...] + (1, 1) : deploy.sh does not define refuse_unlisted_settings once and call it from the Caddy arm
A3: red at 07af9a78a76533fe7d7515ba4d980ddcacd631f8: AssertionError: Tuples differ: ('set[402 chars] ', 'for name in ${!DECKSTREAK_DEPLOY_@}', 'na[29 chars]NGS') != ('set[402 chars] ', 'refuse_unlisted_settings', 'refuse_unlist[47 chars]_@}') [...] : deploy.sh runs something before the refusal
A1: green at b28d20d8bd3412d0e462ae85f5ab82ecb7cb4bb1
A2: green at b28d20d8bd3412d0e462ae85f5ab82ecb7cb4bb1
A3: green at b28d20d8bd3412d0e462ae85f5ab82ecb7cb4bb1
```
