---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak builder"
---

# A Caddy reload that fails restores the copies kept until it succeeds

## Context and Problem Statement

`deploy.sh caddy-install` and `caddy-remove` move the candidate Caddyfile into place and delete the
previous block before `caddy reload` runs (SPEC-062 R7). A reload that fails then leaves the disk
holding the new files while Caddy runs the old configuration, with no previous copy to restore
(#321). What must the scripts keep, and when, so that the files on the host match the configuration
Caddy runs after a failed reload?

## Decision Drivers

- The running configuration is safe on failure: Caddy "automatically reverts to the last known
  working configuration" when a reload has errors (Caddy documentation, Getting started).
- The next reload, from the systemd unit or from a person, reads the files on disk.
- The script runs as root on the host, so the fix stays shell in the one host script.

## Considered Options (the alternatives it was chosen against)

- **Restore on failure, keeping the copies until the reload succeeds.** Chosen: it is the issue's
  own shape and it needs no new state, only the order of two deletions.
- **Reload the candidate before moving it into place.** Rejected: the block must already sit in the
  configuration directory for the `import` line to read it, so the block still needs a restore, and
  the Caddyfile swap would become a second step that can fail after a good reload.
- **Leave it to the next deploy.** Rejected: the issue names the window, and a reload from the unit
  in between would load the new files that never passed a reload.

## Decision Outcome

Chosen option: "Restore on failure", because it keeps the previous block and Caddyfile until the
reload succeeds, restores both on failure, reloads the restored files, and exits non-zero naming the
reload. A second failure is named distinctly. `caddy-remove` has the same order and gets the same
rule (SPEC-127 R4).

### Consequences

- Good, because the disk and Caddy agree after every outcome but a killed script.
- Bad, because a failed restoring reload still leaves Caddy on its last good configuration and needs
  a person, which the second message says.

### Confirmation

SPEC-127's tests A1 to A5, and the script rows S12701 to S12705.

## More Information

#321, SPEC-062 R7, ADR-062, SPEC-127.
