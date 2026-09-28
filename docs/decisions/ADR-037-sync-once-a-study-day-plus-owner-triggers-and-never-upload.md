---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner, through the maintainer at gate 6), the DeckStreak architect"
---

# DeckStreak syncs once per study day plus the owner's explicit triggers, and never uploads

## Context and Problem Statement

DeckStreak reads the owner's Anki collection by syncing a private copy from the owner's sync server,
with the same Anki login the predecessor uses (ADR-008, ADR-009, SPEC-022). At gate 6 the owner, by
the maintainer's decision, approved reusing that login on four testable conditions:
- (a) no upload path, proven against a fake sync server that records every request;
- (b) at most one sync per day, plus the owner's explicit triggers, so DeckStreak never contends
  with the predecessor's sync or loads the server;
- (c) the credentials read from the secret manager at run time (ADR-038);
- (d) the grant that lets the runtime read them, decided with the host changes at gate 2.

The W0 plan had the predecessor's cadence: a sync every 15 minutes (ADR-027), and jobs that synced
before they ran (SPEC-027's `syncs_first`, SPEC-053 R3). How does DeckStreak meet (a) and (b)
without losing the flagship's morning readings (SPEC-022)?

## Decision Drivers

- The owner's real collection must be untouchable by construction, not by convention.
- The predecessor keeps syncing the same account until cutover (ADR-011), so DeckStreak must stay
  out of its way.
- The morning readings and the digest need the whole previous study day, once, before they run.

## Considered Options (the alternatives it was chosen against)

- One daily sync plus owner triggers, never an upload: chosen, because it meets conditions (a) and
  (b) exactly, and the one daily sync already gives the morning jobs a complete study day. One
  scheduled sync runs per study day after the rollover; the owner's explicit triggers add syncs on
  demand; a recording test proves no upload leaves; the cadence is a setting raised only at cutover.
- The predecessor's 15-minute cadence on offset ticks (the W0 plan): rejected because it breaks
  condition (b) and doubles the load on the owner's server while the predecessor still syncs.
- A sync each time the Mini App opens: rejected because opening the app is not an explicit trigger,
  and it would put the sync count in the viewer's hands.
- Jobs that sync before they run (`syncs_first`): rejected because each such job would add a sync
  to the day. A job reads the study day's sync outcome instead.
- Rely on the engine never being asked to upload: rejected because (a) requires proof by
  construction. A recording server that fails the test on any upload is that proof.

## Decision Outcome

Chosen option.
- **One scheduled sync per study day.** The scheduled `sync` job runs once per study day, at the
  rollover hour, minute 7 (04:07 with the default rollover), clear of the predecessor's sync ticks
  (SPEC-027 R2).
  - It is claimed per study day in the cron-fire ledger, so a second fire, a catch-up or a restart
    does nothing (SPEC-027).
  - Its bounded retries (the predecessor's `SYNC_RETRY_ATTEMPTS`) belong to that one run.
- **Owner triggers.** The only other syncs are the owner's explicit ones: the bot's `/sync`
  (SPEC-026), and any later Mini App action that names itself as one.
  - `sync_runs` records each run's trigger, `scheduled` or `owner`.
  - An owner trigger within 5 minutes of a successful sync returns that result without syncing.
  - The collection lock serialises every run (SPEC-022 R7).
- **No job adds a sync.** The readings generation (SPEC-053), the digest and the settle step read the
  study day's sync outcome: a sync that started in the study day and succeeded, scheduled or
  owner-triggered.
  - After a restart, the day's scheduled sync may not have run yet. The `sync` job is caught up
    within the scheduler's lateness bound. A job that needs the outcome first runs that same one
    scheduled sync itself, claimed for the study day under the collection lock, so the count never
    exceeds one and the reader always follows the sync.
  - When the outcome did not succeed, they record `sync_failed`, and the owner can trigger a sync and
    regenerate (SPEC-048).
- **No upload path.** A test drives every sync scenario (a normal sync, a full-sync demand, an empty
  server) through a recording fake sync server. It fails on any full-upload request, and on any
  request that carries a local change. On a full-sync demand the client downloads, or aborts with
  `full_upload_required` when the server holds no collection; it never uploads.
- **The cadence is a setting whose default is one per study day.** Raising it is decided at
  cutover (#164), when the predecessor stops syncing, and never before.

### Consequences

- Good, because the owner's collection cannot be replaced by DeckStreak, and the predecessor's
  sync is never contended.
- Good, because each morning job reads one complete study day.
- Bad, because intra-day views (today's reviews, a celebration when a session ends) wait for the
  owner's trigger, or for the next day. During side-by-side the predecessor still serves those
  moments, and cutover (#164) revisits the cadence with the evidence.

### Confirmation

- SPEC-022's recording-server census and its per-study-day refusal.
- SPEC-027's job table test, which holds `sync` to one daily slot, and SPEC-053's rule that the
  generation never syncs.

## What would make this wrong

- The predecessor is retired (cutover). The cadence can then rise, decided with the evidence at
  #164.
- The owner's server is replaced by one that throttles differently. The daily slot and the debounce
  are then revisited against that server's documented limits.

## More Information

SPEC-022 (the sync), SPEC-026 (`/sync`), SPEC-027 (the scheduler), SPEC-053 (the readings jobs);
ADR-008, ADR-009, ADR-011, ADR-027 (its `sync` row is superseded by this ADR), ADR-038.

Amended in part by SPEC-027 as delivered (#224, 2026-09-28): no job but `sync` runs the sync cycle.
The sentence above that a job needing the outcome "first runs that same one scheduled sync itself"
no longer holds: such a job reads the study day's sync outcome and never runs a sync (SPEC-027 R5,
A12). After a restart, the `sync` job's own catch-up (SPEC-027 R1) is the only run of a missed
scheduled sync, and a job that runs before it has caught up reads the study day's outcome as not
succeeded, unless an owner-triggered sync has succeeded that day, and records it, as its own SPEC
says; the owner can trigger a sync and regenerate. The count of scheduled syncs per study day stays
at most one.

Amendment (2026-09-28): one passage stating the predecessor's schedule as a running service, in
the decision outcome, was redacted under the public-prose rule (ADR-059).
