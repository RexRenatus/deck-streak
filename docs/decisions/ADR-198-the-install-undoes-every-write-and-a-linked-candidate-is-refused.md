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

- Refuse, before any write, a path that exists and is not a plain file with one link — chosen, because it leaves the file state as found, prints the refusal or the write's message and is one test line per script before any write; `cp -p` cannot stand in for it, because it writes through a link to any other existing file and refuses only a link to its own source or to nothing.
- Remove the link and go on — because it deletes a file the script did not create and hides that a stranger put something at its path, it lost.
- Write the candidate with `mktemp` in the same directory — because a fresh name removes the shared path but adds a second cleanup and a name the tests and rows cannot anchor on, it lost.
- For the install, an explicit `|| undo` on each write — chosen, because each write's failure is then visible in the script at the line that can fail and each has its own row and test.
- For the install, one `trap` on exit that undoes — because a trap also fires on the success path and on the `mv` steps after the point of no return, so it would need a flag to tell them apart, it lost.
- For the install, a guard on the candidate path alone — because the block, the block's previous copy and the Caddyfile's previous copy are written through the same way, so a guard on one name leaves three, it lost.

## Decision Outcome

Chosen option: "refuse a path that is not a plain file with one link" for both scripts and "an explicit
`|| undo` per write" for the install, because each keeps the script's control flow readable and gives
every guard one test and one mutation row. The install checks its four fixed names (the block, the
block's previous copy, the candidate and the Caddyfile's previous copy) before its first write, and
the removal checks the same four names; both scripts also refuse, before any write, a Caddy directory
they cannot write and a live Caddyfile that is not a regular file. The undo tolerates an absent candidate and an absent block, never
deletes a block path that is not a file, and its helpers are defined before the first write.

### Consequences

- Good, because every early exit of the install prints the refusal and restores the previous block; the removal makes no such promise for every exit and prints the write's own message when its four-name guard refuses.
- Good, because a link, a directory, a pipe or a file with a second link at a path the install writes
  changes nothing: the run refuses first.
- Good, because the removal refuses a candidate that is a link, a pipe, a socket, a device or a file
  with a second link, and deletes nothing for a pipe, a socket, a device or a second link.
- Bad, because the removal's cleanup after its write's own refusal deletes a link found at the run's
  own candidate name, when that link points to a regular file (`find -delete` removes the link, never
  its target); a dangling link, a link to a directory and a directory are left in place.
- Bad, because the install's refusal message is the same whatever the path shape, so the operator
  reads the path from the listing.

### Confirmation

SPEC-127 criteria A16 to A37 in `scripts/tests/test_deploy_scripts.py`, and rows S12718 to S12735,
each proved killed by its full id.

## More Information

#423, #424, SPEC-127, ADR-127, and the review of #415.
