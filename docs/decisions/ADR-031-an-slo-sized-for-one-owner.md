---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The API's SLO is sized for one owner's traffic, and the alert path never puts the bot token on a command line

## Context and Problem Statement

The observability pack requires every HTTP service to have an SLO with an error budget policy and
multiwindow burn-rate alerts, delivered on one Telegram alert path, and its templates give a
starting point: 99.5% over 28 days, a page at 2% of the budget in one hour (13.44x). DeckStreak's API
serves one owner: a few hundred requests on a study day, none overnight. The template's alert
script also passes the bot token inside the URL on `curl`'s command line. SPEC-031 declares the SLO
and builds the alert path; what objective and windows fit one owner, and how does the script keep
the token off argv?

## Decision Drivers

- An alert must mean something at this traffic: the SRE workbook's low-traffic guidance (group,
  synthesise traffic, lengthen the window, or lower the objective).
- The burn-rate arithmetic the pack checks: burn = budget consumed × window hours / long-window hours,
  and burn × (1 - objective) at most 1.
- CHARTER 15: no secret on argv; `/proc/<pid>/cmdline` is readable by other local users.
- One alert path for every page (the pack's `obs.alert-route`).

## Considered Options (the alternatives it was chosen against)

- Objective 0.99 over 28 days; a page at 5% of the budget over 6 hours (burn 5.6, short window 30 minutes) and a ticket at 10% over 3 days (burn 0.9333, short window 6 hours); `low_traffic: longer-window` — chosen: at about 20 requests an hour, the page needs a sustained failure rather than two unlucky requests, and the numbers pass the pack's arithmetic.
- The template's 99.5% with a one-hour page (13.44x) — rejected because at this traffic two failed requests inside an hour exceed the page threshold, so it would page on noise.
- Synthetic probe traffic to fill the denominator — rejected because always-successful probes dilute the owner's own failures, making a real outage look smaller than it is.
- No SLO for the API — rejected because the pack's `obs.slo-declared` blocks it, and the owner would learn of a degraded API only by using it.
- The template's script, with the token in the URL on `curl`'s command line — rejected because the token is readable in the process table while the request runs.
- The URL given to `curl` through a configuration read from standard input (`--config -`) — chosen: `curl` documents configuration from a file or standard input, and neither appears in the process table.

## Decision Outcome

Chosen options as above. `deploy/slo.json` carries the one SLO; the evaluator and the memory watch
page by failing their units, once per episode, through `deck-streak-alert@.service`, which is also
every unit's `OnFailure=` target. A latency SLO waits until the host has measured the API's real
latency (W2).

### Consequences

- Good, because a page means the owner's API has been failing for tens of minutes, not for a moment.
- Good, because the token stays in `$CREDENTIALS_DIRECTORY` and a pipe.
- Bad, because a total outage pages after about twenty minutes rather than five; the units' own
  failures and the dead-man watch page first for the outages that stop a process.

### Confirmation

SPEC-031's A2 to A5 and the observability rows, enforced in `scripts/check.sh`.

## What would make this wrong

- The API's traffic grows by an order of magnitude (then the template's faster page becomes
  meaningful).
- The first month on the host shows the budget spent by causes the owner does not notice (the
  objective is then too strict for what matters).

## More Information

SPEC-031; the observability pack and its `slo.template.json` and `alert-telegram.template.sh`; the
SRE workbook's "Alerting on SLOs"; curl's `--config` documentation; `docs/schematics/alert-and-slo-path.md`.
