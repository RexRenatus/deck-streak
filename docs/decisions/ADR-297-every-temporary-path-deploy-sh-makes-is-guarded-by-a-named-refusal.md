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
`tar`, `ln`, `mv`, `install`, `cp` or `find -delete`, or a directory that cannot take the write, ended the script under `set -e` with
the tool's own message and no `deploy:` line, so the operator could not tell which deploy step
stopped, and some failures came after the unit files and the daemon reload had been written
(SPEC-127 amendment of 2026-09-30, #451). How does every such failure end a verb with one named line
and every path as it was?

## Decision Drivers

- The refusal line is the only way the operator learns which step stopped.
- A step that cannot make its temporary path must leave the host as it found it.
- A new tool, not only a new call, that the host step writes with must turn the test red.

## Considered Options (the alternatives it was chosen against)

- Guard each path's tool inline in one house form, check the directories before any write to the host's paths, make the pid-named link before the unit files, and delete what was made on every exit — chosen, because each failure is visible at the line that can fail, every guard has its own row, and the failure of the link leaves no unit and no reload to undo.
- Guard only the two sites the verification named, the partial directory and the pid-named link — because the same failure reaches the releases directory, the unpack, the rename into place and a directory the units are written in, it would leave those bare, it lost.
- One `trap` on exit that prints a generic line — because a trap also fires on a success, cannot name the step and cannot tell a failed tool from any other failure, it lost.
- A start-up check that the release root is writable, and nothing else — because it cannot see the unit directory, a drop-in directory or the temporary directory, nor a tool that cannot run, it lost.
- Check `TMPDIR` once at start-up — because it cannot see a `mktemp` that is unrunnable or a directory that fails later, and the host script reads a different `TMPDIR` from the controller's, it lost.
- One shared helper that makes the path and dies — because the host script is a separate shell and cannot call the controller's helper, so the form would be written twice anyway, it lost.
- Keep the link where it was, after the unit files, and add an undo of the units — because with no previous release the install leaves units the existing undo does not remove, it adds writes to undo instead of removing the need, it lost; the link is now made before the units, and the undo that remains is for what can only be tried after them.
- Undo a failed switch by removing the new release's units only — because a rollback from a host that already holds units has units whose bytes differ from the release's, so a removal leaves a path that is not as it was, it lost against a copy of the unit files saved before the first change to a unit file and put back on the undo.
- Save the unit files as a `cp -a` copy in a directory, or in a second `mktemp` directory — because a `cp -a` of a read-only drop-in directory leaves a saved directory the trap cannot delete, and a second `mktemp` would add a site to the ones the test of A40 reads, it lost against one `tar` archive named beside the check file, which the same exit trap removes.
- Put inline guards at each undo site — because a guard repeated at each of several failing calls drifts, it lost against one `undo_and_refuse` that every failing call after the first change goes through.
- Restore a partly deleted stale unpack when its `find -delete` fails — because a half-restored unpack is not the unpack that was there, it lost against a check that the releases directory takes a write before the stale unpack is touched.
- Leave a first install's release root where the run made it — because a refused first install would leave a root that did not exist, it lost against recording the topmost missing directory and removing it on every exit of a run that did not finish.

## Decision Outcome

Chosen option: "guard each path's tool inline in one house form, check the directories before any write
to the host's paths, make the pid-named link before the unit files, and delete what was made on every exit".
The controller's calls end with `|| die "..."`. The host script's guards end with `refuse "..."`,
which prints `deploy: the host step ...` and exits 1. The host script checks the unit directory and
every unit drop-in directory before it writes any path of the host (the check file's `mktemp` comes first), stages the pid-named link before
the unit files are installed, saves the unit files it may replace into one archive named beside
the check file, and an `EXIT` function deletes the check file, the saved archive, the link, an
unfinished unpack and, when the run did not finish, the topmost directory the run had to make. Every
`install` and `find -delete` of the unit installation ends with `|| return 1`, and a failure after the
first change to the host (a failed rename over `current`, a unit that cannot be installed, a check
file that cannot be deleted, a refused effective configuration) removes every unit file, puts the
saved ones back, reloads the daemon, removes an install's new release and refuses. Each line names its step (the release step, the Caddy
step or the host step).

### Consequences

- Good, because every failed write the tests reach ends with one `deploy:` line, a non-zero exit and every fixture path as it was, the release root a first install made and a unit only the new release ships included.
- Good, because the test measures the directories a verb writes in from a real run and fails each, so a new write location in a measured run turns the test red until its expectation names it; a path the measured runs do not reach is not covered. The test of A42 reads the host body for every command that can write a path and turns red on a tool it neither fails nor names as unreached, so a new tool needs a test edit that says so.
- Bad, because the host form repeats what `die` does, since the host script cannot reach `die`.
- Bad, because the directory pre-check reads permissions before the writes, so a directory that changes between the check and the write is not seen by it; the proof of that check-then-act surface is follow-up #505.
- Bad, because the saved archive adds a write before the first change to the host.

### Confirmation

`test_deploy_scripts.py` `every_temporary_path_that_cannot_be_made_is_a_named_refusal` runs every
derived `mktemp` call under every verb that reaches it and each failure mode, and
`every_place_a_verb_writes_in_and_every_temporary_path_call_is_refused` measures the directories of
the release and both rollbacks, fails each in turn and fails each call the host step makes to `mktemp`,
`mkdir`, `tar`, `ln` or `mv`; `every_state_and_every_writing_call_of_the_host_step_is_refused` reads
the host body for its writing tools and fails each call, in each verb, from each of four states; rows
S12747 to S12792 pin the guards; the `mutation-rows` job of CI measures each row.

## More Information

Issue #451; SPEC-127 (amendment of 2026-09-30); ADR-198.

## Amendment, 2026-10-01: round three of the fix; corrections as old/new pairs

The text above is not edited. Each sentence it corrects is quoted as `old`; `new` governs.

- old (Consequences): "every guard has its own row". new: the guards that had none now have
  one (S12793 to S12799 and S12793-B); the second `check_dirs` call is a pre-check whose directories
  the first call already reads, so it has no row of its own.
- old (Decision Outcome): "before it writes any path of the host (the check file's `mktemp` comes
  first)". new: the check file's `mktemp` comes first, then the saved archive of the unit files,
  and only then the directory check; the archive is the one write that precedes it.
- old (Consequences): "every failed write the tests reach ends with one `deploy:` line, a non-zero
  exit and every fixture path as it was". new: every failed write of every state of the registry
  ends so, the trap's and the prune's removals included, and a run that already finished its switch
  ends 0 with no temporary path and no release a later verb accepts while half deleted. Nothing
  is filtered out: a filter that drops a member is a weakening, not a bound.
- old (Consequences): "The test of A42 reads the host body for every command that can write a
  path". new: it reads every operator, substitution, backtick, trap string and reader option, and
  names an unknown word; 26 escaping spellings are planted bodies, each red by assertion.
- old (Confirmation): "...reads the host body for its writing tools and fails each call, in each
  verb, from each of four states". new: it fails every writing call, `rm` and the trap's and
  prune's `find` included, in each verb, from each state of one registry held at a floor.
- Decision added: each removal the trap and the unwind make is written twice, joined by `||`, with a
  final `|| :` in the trap. Chosen against a generic `retry "$@"` helper, which would hide the
  command from the census that reads the body, and against `set +e` in the trap, which hides the
  failure it is there to survive. A stale unpack is set aside and put back by the trap on a refused
  run, chosen against deleting it first, which a refusal could not undo. The prune asks whether a
  release can be deleted whole before it deletes, chosen against deleting and warning, which leaves
  a half-deleted release a later rollback could accept.
- Confirmation: `every_state_and_every_writing_call_of_the_host_step_is_refused` and the
  double-fault tests of the undo and the way back; rows S12793 to S12799 and S12793-B.
