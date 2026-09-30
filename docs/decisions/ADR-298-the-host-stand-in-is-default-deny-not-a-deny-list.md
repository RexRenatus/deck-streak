---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The deploy tests' host stand-in is default-deny, and the setting is named by one helper

## Context and Problem Statement

The stand-in the deploy tests use for the deploy host ran its arguments unconditionally. The deploy
script's elevation setting falls back to a privilege command when it is unset, so a test
environment built without the setting would make the stand-in run that command (#502). How does the
stand-in decide what it runs, and how do the tests make sure the setting is always named?

## Decision Drivers

- A test double must fail closed on a command it was not written for, for any command and not only
  the ones someone thought of.
- The shapes the tests legitimately use are few and known, and a new one should be a reviewed
  change.
- Production behaviour is not this delivery's to change.
- The deploy tests run only in CI, so the evidence must come from CI.

## Considered Options (the alternatives it was chosen against)

- Default-deny: the stand-in runs only the `argv[0]` values in one list, refuses every other with
  one line naming it, and the list is proved equal to the set the tests make: chosen, because it
  refuses a command nobody listed, and a new shape fails a test until it is added on purpose (#502).
- A deny list of privilege commands in the stand-in: rejected, because it refuses only the names
  someone thought to write, so another privilege command, a wrapper or an unthought-of spelling
  still runs, and the list itself would be a second place that has to stay current (#502).
- Setting `DECKSTREAK_DEPLOY_ELEVATE` in the shared environment only: rejected, because that is
  the state that already holds and it protects nothing against an environment built elsewhere; two
  calls in the Caddy test build their own (#502).
- Changing the fallback in `deploy/deploy.sh`: rejected, because the fallback is production
  behaviour the deploy relies on, and this delivery is about the test double (#502).
- Deriving the allowed set from the logged `argv[0]` alone: rejected, because a set read only from
  what ran would accept whatever the tests start to run; the list is compared with both the log and
  the script's own host call and the test module's elevation values (#502).
- Refusing an unnamed environment with a default that fills the setting in: rejected, because a
  default hides the gap that the refusal is there to show (#502).

## Decision Outcome

Chosen option: default-deny. `HOST_ALLOWED` is the one list in the test module; the stand-in
compares `argv[0]` to each entry as a string, so a glob or a path prefix never matches, and an empty
list refuses everything. The stand-in logs each `argv[0]` it receives, and a test compares the set it
logged over a deploy, a second deploy and a rollback with the list and with the set derived from the
script. `launch` is the one place a deploy script starts; it raises naming the setting when the
environment does not set it, reading a mapping, a list of entries and a sourced file. A census over
the module's syntax tree fails any other call that starts a program.

### Consequences

- Good, because a command the stand-in was not written for is refused by name and never run.
- Good, because a test environment without the setting cannot start a deploy script.
- Bad, because a new legitimate host shape needs a list entry; A1 names it when it is missing.

### Confirmation

`scripts/tests/test_deploy_standin.py` (SPEC-298 A1 to A9) and the rows S29800 to S29808, each proved
by the mutation job.

## More Information

Issue #502; the dispatch-shard stand-in has the same rule under #497.
