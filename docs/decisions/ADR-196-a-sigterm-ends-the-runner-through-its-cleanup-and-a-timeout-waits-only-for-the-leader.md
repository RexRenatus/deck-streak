---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# A SIGTERM ends the runner through its cleanup, and a timed-out killer is waited for only at its leader

## Context and Problem Statement

The row runner starts each killer as the leader of a process group of its own (SPEC-025 section 8,
issue 366), so a timeout or an interrupt ends the killer with everything it started. That put the
killer outside the runner's group. A SIGTERM of the runner, to its pid or to its whole group as
`timeout -s TERM` and a job cancel send it, now ends the runner with no cleanup and leaves the
killer's group running, where before the change the group signal reached the killer too. After a
timeout the runner also collected the killer's output, and a descendant that left the group and
still held the pipes kept that collection waiting until it exited, although the timed-out result
discards the output. The pipes were also left open on the interrupted path (issue 409). How does the runner end the killer's group on every signal, return at its bound,
and close its pipes?

## Decision Drivers

- A signal that ends the runner must end what the runner started, as an interrupt already does.
- A timed-out run returns at its bound whatever a descendant outside the group holds.
- No new dependency and no `unsafe`: the runner is standard-library Python.
- The change stays small enough to pin with one test per edge and one row per behaviour.

## Considered Options (the alternatives it was chosen against)

- Keep the killer in the runner's own group — rejected because section 8 left it for issue 366's
  reason: a group kill on timeout would then kill the runner too, and the group kill is what ends
  a hung test binary's daemons.
- A SIGTERM handler that signals the killer's group itself — rejected because it duplicates the
  cleanup that `run_in_own_group`'s `finally` already performs, and a handler that acts before the
  runner unwinds races the leader's reaping.
- `PR_SET_PDEATHSIG` set in the killer through `preexec_fn` — rejected because it fires when the
  runner's thread dies rather than when it is signalled, it is not inherited past the leader, and
  `preexec_fn` is unsafe with threads.
- `sys.exit(128 + signum)` from a SIGTERM handler, so the exception unwinds through the existing
  `finally` blocks — chosen because it reuses the one cleanup the interrupt path already has, adds
  no state, and leaves a shell-conventional exit status.
- Keep `communicate()` after the kill and bound it — rejected because the output of a timed-out run
  is discarded, so collecting it is unneeded work that an escapee can hold.
- `wait()` for the leader only on the timed-out path, and close both pipes in the `finally` —
  chosen because it returns at the bound, needs no timeout of its own, and closes the pipes on
  every way out.

## Decision Outcome

Chosen option: "a SIGTERM handler that exits through the cleanup, with a leader-only wait on
timeout and an explicit pipe close", because it ends the killer's group on SIGINT and SIGTERM
through one path, returns at the bound, and closes every pipe, in about a dozen lines.

### Consequences

- Good, because a `timeout -s TERM` or a job cancel leaves no killer group running.
- Good, because a run cannot outlast its bound on a held pipe.
- Good, because no path leaves a pipe open.
- Bad, because a descendant that left the group is still neither ended nor waited for.
- Bad, because `main()` sets a process-wide SIGTERM disposition, which is right for the command
  line and would surprise a caller that imported it.

### Confirmation

`scripts/tests/test_mutation_rows_group.py` runs the runner as a process, signals it, and asserts
by pid that the killer and its grandchild are gone; asserts a timed-out killer with an escapee
returns within its bound plus a stated margin; and asserts both pipes are closed on each path.
Rows S02506 to S02508 prove each behaviour KILLED.

## More Information

Issue 409, following issue 366; SPEC-025 sections 10 and 11; ADR-025.
