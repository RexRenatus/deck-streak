The owner admits `mutation-rows.json#S06226-TWO-INSTANCES-OF-ONE-TEMPLATE-ARE-REFUSED` (ROW-REMOVED; its target `scripts/tests/test_deploy_templates.py` stays), in the owner's own words: "Sign the ruling (Recommended)" (owner, 2026-10-01): a second instance drop-in of one shipped template is admitted only for the allowlisted `sync` and `held_flush` instances, every other instance stays refused, and each credential pair is pinned to its own instance by rows S06229-S06234.

# OWNER RULING 2026-10-01: the held-flush instance allowlist

## What was held

Row S06226 pinned that a shipped job template has exactly one instance drop-in directory. At the
merge-base its anchor was `if template is not None and instances[template] == 1:` at
`scripts/tests/test_deploy_templates.py:431`, its mutant relaxed that to `if template is not None:`,
its killer was
`test_deploy_templates.CredentialsComeFromTheSocket.test_a_shipped_templates_instance_dropin_directory_is_its_own_and_no_other_is`,
and its stem read `TWO-INSTANCES-OF-ONE-TEMPLATE-ARE-REFUSED`. The row sits at
`scripts/mutation-rows.d/S06200-S06299.json:205`. The scheduled held flush needs a second instance
of the template, so the row's meaning no longer holds as written.

## What replaces it

One allowlist, `INSTANCE_DROPIN_ALLOWLIST`, kept in one place in the deploy-template tests. An
instance drop-in directory of a shipped template is admitted only for `sync` and `held_flush`;
every other instance is refused by default. Rows:

- S06229: an unnamed instance drop-in of a shipped template is refused.
- S06230: an unshipped template's instance drop-in is refused even when its name is allowlisted.
- S06231: the sync instance never loads the held flush's credential pair.
- S06232: the job template itself never loads the held flush's credential pair.
- S06233: the held flush instance never loads the sync login pair.
- S06234: the credential lister names the sync pair under the sync unit only.

S06226 is retired in `scripts/mutation-rows.retired.json`; no id is re-meant.

## Why it is admitted

The merged-template view that the tests read is not what systemd does: it judges each instance on
its own, so per-instance exactness, with each credential pair pinned to its one instance, is a
stronger statement than a single-directory count. The allowlist is default-deny, not a removal.
ADR-300 names the two rejected alternatives: a separate job template for the flush (it duplicates
the template's hardening and unit guards, doubles the deploy surface and loses the per-instance
exactness), and routing the flush through the long-running bot service (it loses the job ledger,
the timer's catch-up and its own failure record).
