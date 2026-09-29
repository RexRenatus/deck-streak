---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The Caddy install undoes every write it made, and a linked candidate is refused

## Context and Problem Statement

`deploy.sh caddy-install` and `caddy-remove` run as host scripts under `set -eu`, so a failing
command that nothing guards ends the script with no message. The install's undo began with a `find`
that failed when the candidate was absent, and three of its writes had no undo. The removal's write
of its candidate followed a symbolic link, so a link at that path could empty the live Caddyfile
or put itself in its place (SPEC-127 amendment of 2026-09-29, #423 and #424). How does each script
stop safely at every early exit?

## Decision Drivers

- The refusal message is the only way the operator learns which deploy step stopped.
- The live Caddyfile must never be changed by a write that was meant for the candidate.
- The fix must not change what a successful run does.

## Considered Options (the alternatives it was chosen against)

- Refuse a candidate path that is a link, and go no further — chosen, because it leaves the file state as found, prints the write's message and needs one test line before any write.
- Remove the link and go on — because it deletes a file the script did not create and hides that a stranger put something at its path, it lost.
- Write the candidate with `mktemp` in the same directory — because a fresh name removes the shared path but adds a second cleanup and a name the tests and rows cannot anchor on, it lost.
- For the install, an explicit `|| undo` on each write — chosen, because each write's failure is then visible in the script at the line that can fail and each has its own row and test.
- For the install, one `trap` on exit that undoes — because a trap also fires on the success path and on the `mv` steps after the point of no return, so it would need a flag to tell them apart, it lost.

## Decision Outcome

Chosen option: "refuse a linked candidate" for the removal and "an explicit `|| undo` per write" for
the install, because each keeps the script's control flow readable and gives every guard one test
and one mutation row. The undo tolerates an absent candidate and its helpers are defined before the
first write.

### Consequences

- Good, because every early exit of the install prints the refusal and restores the previous block.
- Good, because a link at the removal's candidate path changes nothing the run did not own.
- Bad, because the removal's cleanup deletes a link to a regular file (`find -delete` removes the
  link, never its target) and leaves a dangling link, a link to a directory and a directory in place.
- Bad, because a failed copy of the previous block, and a read-only Caddy directory holding a stale
  candidate, still fail before any undo exists and print no refusal (#423).
- Bad, because the install has no link guard of its own and relies on `cp -p` refusing.

### Confirmation

SPEC-127 criteria A16 to A21 in `scripts/tests/test_deploy_scripts.py`, and rows S12718 to S12722,
each proved killed by its full id.

## More Information

#423, #424, SPEC-127, ADR-127, and the review of #415.
