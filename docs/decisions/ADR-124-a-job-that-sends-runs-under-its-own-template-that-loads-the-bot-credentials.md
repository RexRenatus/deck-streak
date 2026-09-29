---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 fix round)"
---

# A job that sends runs under its own template, which loads the bot's two credentials

## Context and Problem Statement

The job role builds no bot transport and loads no bot token: "no job of the W0 table sends a
message, so the role builds no bot transport and loads no bot token, and the first job that sends
joins wiring's `TransportMarker`" (`crates/daemon/src/role_job.rs`, its module comment; SPEC-026
R10; SPEC-041 R13). Without a transport the router withholds every bot occasion as
`Withheld{NoNotifier}` before any push (`crates/notifications/src/router.rs`, `decide`).

W5 adds the first jobs that send: `morning_nudge`, `evening_nudge`, `last_chance_nudge`,
`daily_digest`, `weekly_report`, `widget_refresh` and `discipline_tick` (SPEC-100, SPEC-101,
SPEC-102, SPEC-105). Their tests run over a recording transport, so a build that follows the
manifests would pass every test and record `no_notifier` for every nudge and digest in production.

A credential reaches a unit only as `LoadCredential=` (`deploy/README.md`), and the template
`deck-streak-job@.service` is shared by every job, the liveness and maintenance jobs included.
SPEC-062 R14 keeps the sync login off that template in a drop-in of the `sync` instance's own
`.service.d` directory, and its A21 refuses a second instance directory of one template.

Where do the bot's two credentials, `owner-user-id` and `telegram-bot-token`, reach a job that sends?

## Decision Drivers

- ADR-038: a credential is read from the private rail's socket at every start and never stored.
- The least privilege of the rail's map: an instance is answered only the credentials it needs.
- SPEC-062 R14, A21 and A22: the unit guards admit one instance directory per template, and an
  instance's drop-in sets only `LoadCredential=`.
- `deck-streak-alert@.service` already holds exactly these two credential lines.
- Seven sending jobs must not become seven hand-copied units.

## Considered Options (the alternatives it was chosen against)

- A second job template, `deck-streak-job-send@.service`, identical to `deck-streak-job@.service` but for the alert template's two `LoadCredential=` lines, with each sending job's `@<id>.timer` on it — chosen, because the guards need no change, the rail's map names one template for the two ids, and a test can pin that the two templates differ by those two lines alone.
- A drop-in in each sending instance's `.service.d` directory — rejected because SPEC-062 R14 and A21 refuse a second instance directory of one template, and admitting several reopens a guard that was verified over six rounds.
- A template-level drop-in, or the two credential lines on `deck-streak-job@.service` — rejected because every job, the liveness and maintenance ones included, would then hold the bot's token.
- A separate unit for each sending job — rejected because seven hand-copied units drift apart, each needing its own budget and contract entries.

## Decision Outcome

Chosen option: "a second job template that loads the bot's two credentials", built by SPEC-100.

- **The template.** `deck-streak-job-send@.service` equals `deck-streak-job@.service` in every
  key but the two `LoadCredential=` lines `owner-user-id` and `telegram-bot-token`, in the form of
  `deck-streak-alert@.service`. A test pins that the two differ by those lines alone.
- **Which jobs.** The job table marks each job as sending or not. A sending job runs as an
  instance of the sending template, with its timer `deck-streak-job-send@<id>.timer`; a job that
  sends nothing stays on `deck-streak-job@` and requests neither credential.
- **The role.** The job role joins the bot's transport to the router before a job that sends runs:
  it loads the two credentials through its credential loader, builds the transport and passes
  wiring's `TransportMarker` as the notifier. A sending job started without either credential
  refuses start, and a job that sends nothing builds no transport.
- **The rail.** The rail's map answers the two ids to the sending template's instances alone, in
  the form of SPEC-061 R4.

- **The budget.** `deck-streak-job-send@.service` equals the job template's ceilings, in the row
  below, which `deploy/host-budget.json` holds equal and the deploy-template test's `adr_budget()`
  reads beside ADR-032's and ADR-064's tables.

The sending template's ceilings, in the form of ADR-064's table:

| unit | MemoryHigh | MemoryMax | CPUQuota |
|---|---|---|---|
| `deck-streak-job-send@.service` | 320M | 384M | none (a job) |

### Consequences

- Good, because a sending job records `sent` in production and not `no_notifier`.
- Good, because the liveness, maintenance and sync jobs never hold the bot's token.
- Good, because the unit guards keep every rule of SPEC-062; they learn the second template's name,
  credentials and budget, and the pin test holds the two templates equal.
- Bad, because two templates must not drift, which the pin test guards.
- Bad, because the template needs its own entries in the host budget and the rail contract.

### Confirmation

SPEC-100's A47 (a sending job routes through the joined transport and a job that sends nothing
builds none), A48 (a sending job started without either credential refuses start) and A49 (the two
templates differ by the credential lines alone), and the rows `S10022` and `S10023`.

## What would make this wrong

- A W5 job that sends must run under a template that carries a setting the sending template lacks:
  the two templates would then diverge, and the pin test would refuse it.
- The rail's map cannot answer by template: the ids would then need an instance-level answer.

## More Information

Cites ADR-027 (jobs are timers with a ledger), ADR-032 (the host budget), ADR-038, ADR-061,
SPEC-026, SPEC-041, SPEC-061, SPEC-062 and SPEC-100.
