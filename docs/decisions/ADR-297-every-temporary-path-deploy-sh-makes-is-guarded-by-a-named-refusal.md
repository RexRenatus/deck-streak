---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# Every temporary path `deploy.sh` makes is guarded by a named refusal

## Context and Problem Statement

`deploy.sh` makes temporary paths with `mktemp` (the release step's working directory, the Caddy
install's working directory and the host script's check file) and without it (the host script's
`releases/<tag>.partial` directory and its pid-named `.current.<pid>` link). A failed `mktemp`, `mkdir`,
`tar`, `ln` or `mv`, or a directory that cannot take the write, ended the script under `set -e` with
the tool's own message and no `deploy:` line, so the operator could not tell which deploy step
stopped, and some failures came after the unit files and the daemon reload had been written
(SPEC-127 amendment of 2026-09-30, #451). How does every such failure end a verb with one named line
and no write?

## Decision Drivers

- The refusal line is the only way the operator learns which step stopped.
- A step that cannot make its temporary path must leave the host as it found it.
- A new `mktemp` call must be caught by the test without anyone editing it.

## Considered Options (the alternatives it was chosen against)

- Guard each path's tool inline in one house form, check the directories before the first write, make the pid-named link before the unit files, and delete what was made on every exit — chosen, because each failure is visible at the line that can fail, each guard has its own row, and the failure of the link leaves no unit and no reload to undo.
- Guard only the two sites the verification named, the partial directory and the pid-named link — because the same failure reaches the releases directory, the unpack, the rename into place and a directory the units are written in, it would leave those bare, it lost.
- One `trap` on exit that prints a generic line — because a trap also fires on a success, cannot name the step and cannot tell a failed tool from any other failure, it lost.
- A start-up check that the release root is writable, and nothing else — because it cannot see the unit directory, a drop-in directory or the temporary directory, nor a tool that cannot run, it lost.
- Check `TMPDIR` once at start-up — because it cannot see a `mktemp` that is unrunnable or a directory that fails later, and the host script reads a different `TMPDIR` from the controller's, it lost.
- One shared helper that makes the path and dies — because the host script is a separate shell and cannot call the controller's helper, so the form would be written twice anyway, it lost.
- Keep the link where it was, after the unit files, and add an undo of the units — because with no previous release the install leaves units the existing undo does not remove, so it adds writes to undo instead of removing the need, it lost; the undo that remains is for the rename over `current`, which can only be tried last.

## Decision Outcome

Chosen option: "guard each path's tool inline in one house form, check the directories before the
first write, make the pid-named link before the unit files, and delete what was made on every exit".
The controller's calls end with `|| die "..."`. The host script's guards end with `refuse "..."`,
which prints `deploy: the host step ...` and exits 1. The host script checks the unit directory and
every unit drop-in directory before its first write, stages the pid-named link before
the unit files are installed, and an `EXIT` function deletes the check file, the link and an
unfinished unpack. A failed rename over `current` restores the previous units, reloads the daemon,
removes an install's new release and refuses. Each line names its step (the release step, the Caddy
step or the host step).

### Consequences

- Good, because every failed temporary path ends with one `deploy:` line, a non-zero exit and no write.
- Good, because the test measures the directories a verb writes in from a real run and fails each, so a new write location in a measured run is tested without anyone editing the test; a path the measured runs do not reach is not covered.
- Bad, because the host form repeats what `die` does, since the host script cannot reach `die`.
- Bad, because the directory pre-check reads permissions before the writes, so a directory that changes between the check and the write is not seen by it.

### Confirmation

`test_deploy_scripts.py` `every_temporary_path_that_cannot_be_made_is_a_named_refusal` runs every
derived `mktemp` call under every verb that reaches it and each failure mode, and
`every_place_a_verb_writes_in_and_every_temporary_path_call_is_refused` measures the directories of
the release and both rollbacks, fails each in turn and fails each call to a path-making tool; rows
S12747 to S12771 pin the guards.

## More Information

Issue #451; SPEC-127 (amendment of 2026-09-30); ADR-198.
