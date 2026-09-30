---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# Every temporary path `deploy.sh` makes is guarded by a named refusal

## Context and Problem Statement

`deploy.sh` makes three temporary paths: the release step's working directory, the Caddy install's
working directory and the host script's check file. A failed `mktemp` ended the script under
`set -e` with the tool's own message and no `deploy:` line, so the operator could not tell which
deploy step stopped. The host script's call also came after the unit files and the daemon reload,
so its failure left writes behind (SPEC-127 amendment of 2026-09-30, #451). How does every such
call end a verb with one named line and no write?

## Decision Drivers

- The refusal line is the only way the operator learns which step stopped.
- A step that cannot make its temporary path must leave the host as it found it.
- A new `mktemp` call must be caught by the test without anyone editing it.

## Considered Options (the alternatives it was chosen against)

- Guard each call inline in one house form, and move the host call before the first write — chosen, because each failure is visible at the line that can fail, each call has its own row, and nothing needs undoing.
- Guard only the release step's `mktemp -d`, the one call the issue names — because the Caddy step and the host script fail the same way and one of them fails after a write, it lost.
- One `trap` on exit that prints a generic line — because a trap also fires on a success, cannot name the step and cannot tell a failed `mktemp` from any other failure, it lost.
- Check `TMPDIR` once at start-up — because it cannot see a `mktemp` that is unrunnable or a directory that fails later, and the host script reads a different `TMPDIR` from the controller's, it lost.
- One shared helper that makes the path and dies — because the host script is a separate shell and cannot call the controller's helper, so the form would be written twice anyway, it lost.
- Keep the host call in place and add an undo of the unit files — because with no previous release the install leaves units the existing undo does not remove, so it adds writes to undo instead of removing the need, it lost.

## Decision Outcome

Chosen option: "guard each call inline in one house form, and move the host call before the first
write". The controller's calls end with `|| die "..."`. The host script ends its call with
`|| { echo "deploy: ..." >&2; exit 1; }`, and an `EXIT` trap deletes the check file on every exit.
Each line names its step (the release step, the Caddy step or the host step).

### Consequences

- Good, because every failed temporary path ends with one `deploy:` line, a non-zero exit and no write.
- Good, because the test reads the script for its `mktemp` calls, so a new call joins the population.
- Bad, because the host form repeats what `die` does, since the host script cannot reach `die`.

### Confirmation

`test_deploy_scripts.py` `every_temporary_path_that_cannot_be_made_is_a_named_refusal` runs every
derived call under every verb that reaches it and each failure mode, and rows S12747 to S12752 pin
each guard.

## More Information

Issue #451; SPEC-127 (amendment of 2026-09-30); ADR-198.
