---
status: accepted
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

### Decided at delivery (SPEC-031 §7)

The delivery decided what this record left open, and what SPEC-025's delivery found, each against
its alternatives:

- **A panic is logged by a hook the kernel's one logging setup installs.** `logging::install`
  replaces Rust's default hook once its subscriber is installed, and the hook logs the panic's text,
  its location and its thread as one ERROR event through the redacting writer. Chosen against a hook
  in `main` alone, which every other process that logs, the API's tests among them, would lack;
  against chaining to the default hook, whose plain text on stderr is exactly the leak; and against
  a crate for one function, which no ADR admits.
- **The page is plain text, and the owner's id travels with the token.** The script writes the URL
  and both fields to `curl`'s configuration on standard input from the shell's `printf`, which runs
  no process. Chosen against the chat id on argv, since ADR-038 loads the owner's id as a credential;
  against a here-document, which a shell may write to a temporary file with the token in it; and
  against a parse mode, whose escaping the quoted lines would need.
- **The text is cut at 3500 bytes at a character boundary** by `LC_ALL=C sed`, which drops a UTF-8
  sequence the cut split. Chosen against the template's `cut -c1-3500`, which bounds each line rather
  than the text and, in GNU `cut`, counts bytes and splits characters; and against `iconv -c`, which
  exits non-zero on the cut sequence and would stop the script before the page.
- **The quoted lines are the failed run's**, matched by `$MONITOR_INVOCATION_ID`, and the unit's
  latest only when systemd names no run. Chosen against the unit's latest lines always, which after
  an OOM kill that wrote nothing would quote an older run's.
- **The evaluator reads the journal once per SLO and counts by each entry's own timestamp** against a
  clock a test can fix, with the ratios in exact fractions. Chosen against a read per window relative
  to the host's clock, as the template reads, which cannot take a fixed clock and reads the journal
  once per window; and against float ratios, which can put a ratio at the threshold on either side
  of it. It reads the kernel's flattened event and the formatter's nested one alike.
- **A ticket pages too, once per episode.** Chosen against the template's ticket, printed at priority
  4 and paging no one: with one owner and one channel, a line only the journal holds is read by no
  one.
- **A run that cannot measure is an episode of its own.** Chosen against failing the unit on every
  such run, a page every five minutes, and against succeeding quietly, an unmeasured SLO passing for
  a calm one.
- **The memory watch finds every `deck-streak-*.service` cgroup and keeps each unit's counters with
  its cgroup's inode**, a unit's first sight its baseline. Chosen against the template's list of unit
  names, which misses a template's instances and every unit added later; against counters alone,
  which miss the events of a cgroup a restart made anew when its count equals the old one's; and
  against paging what a unit counted before the watch first saw it.
- **The three units carry SPEC-032's hardening, narrowed.** None reads the settings file, the alert
  writes nothing, the evaluator and the watch keep their episodes in directories of their own and may
  open only `AF_UNIX` sockets, and only the alert and the evaluator join the journal's group. Chosen
  against SPEC-032's settings copied whole: its required `EnvironmentFile=` would stop the alert when
  the settings file is missing, when a page matters most, and its shared state directory would let
  three scripts write where the API keeps its database.
- **The two timers spread their fires and waive the catch-up.** Chosen against waiving
  `timers.spread` as the job timers do, since neither keeps a minute of its own; and against
  `Persistent=true`, since a catch-up run finds nothing a run minutes later lacks, and its stamp would
  be rewritten every minute.
- **A daemon's crash loop pages at each failure.** Chosen against `RestartMode=direct` on the daemons,
  which would page only when the loop ends, and never for a crash a restart heals.
- **`obs.slo-declared` stays deferred to #42.** The probe asks an SLO of every long-running unit whose
  binary serves HTTP, and `deckstreakd`'s bot role shares the API's binary. Chosen against an SLO for
  the bot now, which SPEC-031 excludes until its traffic is measured, and which over the journal would
  count no response and never burn; and against changing the bot's unit so the probe no longer sees
  its binary, which would hide the unit from the check rather than answer it.

### Consequences

- Good, because a page means the owner's API has been failing for tens of minutes, not for a moment.
- Good, because the token stays in `$CREDENTIALS_DIRECTORY` and a pipe.
- Bad, because a total outage pages after about twenty minutes rather than five; the units' own
  failures and the dead-man watch page first for the outages that stop a process.
- Bad, because a daemon's crash loop sends a handful of pages in about a minute before its start
  limit ends it.
- Bad, because the evaluator and the watch run beside the jobs, so every ceiling reached at once
  passes DeckStreak's share, a bound the scripts' real use stays far below (SPEC-031 §6).

### Confirmation

SPEC-031's A2 to A8 and the observability rows, enforced in `scripts/check.sh`.

## What would make this wrong

- The API's traffic grows by an order of magnitude (then the template's faster page becomes
  meaningful).
- The first month on the host shows the budget spent by causes the owner does not notice (the
  objective is then too strict for what matters).

## More Information

SPEC-031; the observability pack and its `slo.template.json` and `alert-telegram.template.sh`; the
SRE workbook's "Alerting on SLOs"; curl's `--config` documentation; `docs/schematics/alert-and-slo-path.md`;
systemd.unit(5) `OnFailure=` and systemd.exec(5) `$MONITOR_*`; systemd's NEWS on `RestartMode=`.
