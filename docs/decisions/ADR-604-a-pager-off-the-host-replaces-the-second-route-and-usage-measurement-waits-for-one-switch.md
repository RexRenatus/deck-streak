---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-604: A pager off the host replaces the second route, the server counts its requests, and usage measurement waits for one switch that is off by default

## Context and Problem Statement

DeckStreak pages its owner through one path. Every service and timer-started job names
`OnFailure=deck-streak-alert@%n.service`, the SLO evaluator and the memory watch page by failing,
and `deploy/slo.json` routes every burn-rate alert to the alert template (SPEC-031 R2 to R5). At
the base this decision was measured on, the template ran a sender that posted to Telegram with
two credentials (`deploy/systemd/deck-streak-alert@.service` lines 17 and 25-26), every alert of
`deploy/slo.json` said `"route": "telegram"` (lines 25, 33, 57 and 65), and `alerting.channel`
said `telegram` (lines 77-80). A second route, its own unit, timer and two credentials, reported a
failed or silent alert sender to a receiver off the host (SPEC-396, ADR-410).

SPEC-582 and ADR-601 take every Telegram surface out of the install. They leave the template's
name and its callers, and give it the journal sender: one line at error priority under the alert
instance's own identifier, opening `DeckStreak: <unit> failed (<result>)`, with the failed run's
last five error lines, no credential and no request (ADR-601 D4). `deploy/slo.json`'s routes and
channel become `journal`. The second route's unit and timer leave; its script, its model and its
tests stay. So once that install ships, a failure is a line in the journal and an entry in
`systemctl --failed`, and nothing tells a person (ADR-601, Consequences).

Three more facts shape what follows. The SLO evaluator prints nothing on a run that has nothing
to say (`deploy/scripts/slo-evaluate.py`, `main`), so no line of its own marks that it ran. The
SLO declaration's schema refuses a key it does not know, so `deploy/slo.json` cannot carry a
pager's declaration. And no app sends any event about how it is used: the web app and the iPhone
and iPad app hold no measurement code, and the app's privacy manifest declares no collected data
type (`ios/App/PrivacyInfo.xcprivacy` lines 9-10, held by `scripts/tests/test_ios_app_tree.py`).

What reads the alert line off the host and tells a person, what replaces the second route, what
the server may count without asking anyone, and how any later measurement of use is chosen?

## Decision Drivers

- No failure is silent: every page the alert template writes reaches a person off the host, and
  a pager that has gone deaf is itself noticed off the host.
- No new credential, no address off the host that DeckStreak must keep, and no vendor.
- The repository declares what pages and what may be counted; the monitoring service's policies,
  channels and settings are made outside it from that declaration, and a hand-run verify on the
  host proves them.
- Logs keep the window `privacy.json` declares wherever a copy of them lives.
- Measuring how the apps are used is off by default, for every learner at every age, behind one
  switch; counting the server's own requests reads nothing from a device and needs no switch.
- Records stay records: a landed SPEC or ADR is kept, and a later record supersedes its
  statements.

## Decisions, and the alternatives each was chosen against

### D1. The pager reads the alert template's one line, and nothing else changes on the host

The pager is a monitoring service off the host that receives the journal's lines of DeckStreak's
own units through a log shipper on the host, matches the lines a page needs, and notifies a
person. Its first page is the alert template's line: a line whose identifier starts
`deck-streak-alert@`, at error priority or more urgent, opening `DeckStreak: `. Every page source
already reaches that line through `OnFailure=`: a unit's own failure, a burning SLO alert, a
ticket, the memory watch, a job's paging transition and a backup's failure. The template, its
sender, its sandbox and every `OnFailure=` line stay exactly as ADR-601 leaves them.

Chosen against:
- A sender that posts to the monitoring service's own endpoint: rejected because it needs a
  credential and a network path for the template, which ADR-601 has just taken away.
- Mail from the host through a relay: rejected because it needs a credential and a relay the
  install does not carry.
- A page through the notification router: rejected because the router serves learners, and an
  alert about the service must not wait on the service.
- Telegram: rejected because the install carries no Telegram surface (ADR-601); no option here
  sends to Telegram.

### D2. What pages, and how a page reaches a person

Four pages, declared in `docs/monitoring/pager.json`:

| page | what it matches | what it tells |
|---|---|---|
| `an-alert-line` | the alert template's line, by its identifier prefix, its priority and its opening | every page the template sends |
| `an-alert-instance-failed` | the service manager's record that an alert instance failed | a sender that could not write its line |
| `the-evaluator-is-silent` | no successful run of `deck-streak-slo.service` recorded for 20 minutes, four of its five-minute periods | a host, a service manager, a timer or a log shipper gone quiet |
| `the-daily-backup-is-late` | no successful run of `deck-streak-backup.service` recorded for 26 hours | a daily backup a day and two hours old |

A page reaches a person through at least two notification channels of two different kinds, one
of them email, so that one channel's outage does not lose a page. Which second kind is the
owner's choice, made with the channels outside this repository.

Chosen against:
- Email alone: rejected because one channel's outage, or one filter, loses every page.
- A text message as the second channel: rejected as the default because its delivery is not
  assured in every region; it stays one of the owner's choices.
- A page into DeckStreak's own app: rejected for now because native push has not landed (#640);
  it joins as a channel when it does.
- A third-party paging service: rejected because it adds a vendor and an address off the host to
  keep.
- One page per burn-rate alert, as a policy of its own on the monitoring service: rejected
  because the evaluator already pages each alert once per episode through the template, so a
  second evaluation would page twice and could disagree with the first.

### D3. What replaces the second route

The second route told the owner when the alert sender failed or went silent. Its replacement is
two of the pager's pages, read off the host by construction. `an-alert-instance-failed` matches
the service manager's own record of a failed alert instance, so a sender that cannot write its
line is still told, by a record it does not write. `the-evaluator-is-silent` and
`the-daily-backup-is-late` match the service manager's record of each successful run, so a host,
a service manager, a timer or a log shipper that stops is told by the silence the monitoring
service hears. A host or an edge that stops answering is the uptime checks' page, made outside
this repository with the same channels. The second route's script, its formal model, its rows
and its tests stay in the tree byte for byte, superseded, until their retirement
(#1103).

Chosen against:
- A heartbeat line the SLO evaluator writes at every run: rejected for this delivery because it
  changes a deploy script, and the service manager's record of each run carries the same signal;
  it returns if the hand-run verify shows that record missing.
- The second route reinstalled with a receiver off the host: rejected because its two receiver
  addresses do not exist, and the pager hears the same failures with no credential.
- The monitoring service's own SLO objects with burn-rate policies: rejected for this delivery
  because they duplicate the evaluator, and their behaviour at one owner's traffic is not
  measured; they are a later option.
- Retiring the script, the model, the rows and the tests now: rejected because the deletion
  reaches deploy and test files outside this delivery's documents and declaration, and removing
  tests needs its own ruling.

### D4. The log store keeps 14 days, set before any line ships, and only DeckStreak's lines ship

Before the log shipper sends its first line, the monitoring service's log store is set to keep a
line for at most 14 days, `privacy.json`'s `logs.retention` (`P14D`), which journald already
keeps (`deploy/journald.conf.d/deck-streak.conf` line 8). The shipper reads only the lines of
units named `deck-streak-*` and the service manager's lines about them. The setting, the shipper
and its filter are acts on the host and the monitoring service, outside this repository; the
declaration states them and `privacy.json` stays as it is.

Chosen against:
- The monitoring service's default retention: rejected because it is longer than the 14 days the
  privacy page states.
- Every line of the host: rejected because DeckStreak's pages need only its own units' lines, and
  another unit's lines may carry what DeckStreak never declared.
- Shipping first and setting the retention after: rejected because a line received under the
  longer default would outlive the stated window.

### D5. The request counters

The server counts the requests its services answer, from the response event the trace layer
already writes for every request of the API and the MCP server (`message` `finished processing
request`, `status`, and the span's `method` and `route`). A counter carries four labels: the unit,
the route's pattern (axum's matched path, so `/api/me`, never the address a request named, and
`unmatched` when no route matched), the method, and the status class (`1xx` to `5xx`). Its
cardinality is bounded by the routers' route patterns, their methods and five classes. It lives
on the monitoring service, derived from the shipped lines, which the log store keeps 14 days; a
counter holds no personal data. Nothing else of the event enters a counter: not the request id,
not a latency, and not any field a later change adds to the span, which the counter test refuses
until this decision is amended.

Chosen against:
- A metrics endpoint in the API: rejected because it is code and a deviation from the house stack
  (#18); the response events are already the metrics.
- A nightly job that counts into the shared database: rejected for this delivery because it is
  code, and the monitoring service counts the same lines with none.
- The request id, the raw path or the client's address as a label: rejected because each is one
  series per request or per person.
- A latency distribution beside the counts: rejected for now because no latency SLO is declared
  (#42); it joins with one.

### D6. Usage measurement is opt-in, off by default, behind one switch

Any measurement of how the apps are used, product events and coarse learning measures alike, is
behind one switch, the same switch for the web app and the iPhone and iPad app, off by default for
every learner at every age. It is shown in each app's account settings and on the privacy page,
and kept with the learner's account, so one choice holds on every device, with when it last
changed. When it is on, an app sends only a fixed list of events (section 9 of SPEC-585) under an
id the device makes anew each study day and never links to the account. Never sent, on or off:
the account, an email or sign-in identifier, a device or advertising identifier, a card, a note, a
deck's name, a search, any free text, a review one by one, an exact time of day and a location.
Turning it off stops the sending at once and forgets the day's id. Until the switch is built, the
apps send nothing (#1100, #1101, #1102).

Chosen against:
- Measured unless the learner objects, where a law allows it: rejected because it needs an age
  gate and region logic DeckStreak does not have, and the same rule must hold for children and
  schools.
- On by default: rejected because profiling and measurement must be off by default for children,
  and DeckStreak has no age gate.
- A switch per app, or one per kind of event: rejected because one switch is what a learner can
  find, understand and trust, and two can disagree.
- No measurement of use ever: rejected because the request counters cannot say which screens
  help, and the learner can be asked.

### D7. Learning measures stay apart from the sealed sync's daily facts

The coarse learning measures the switch covers (a day's review count, minutes and retention, each
in coarse bands) are made on the device and sent only through the switch, under the day's id. They
never ride the sealed sync's record of a day's facts, which serves the learner's own features
under the contract, and the server never reads that record to measure anyone's use.

Chosen against:
- Reading the learning measures from the daily facts on the server: rejected because it changes
  that record's purpose without the learner's choice.
- Sending the learning measures inside the daily facts: rejected because turning the switch off
  could then not stop them, and one record would carry two lawful bases.

### D8. The privacy page and the privacy label state it now, and move with the switch

The privacy page gains a section, `## Measuring use`: the server counts its requests by route,
method and status class, and a count holds no account, address, device, request id or content;
the apps measure nothing about how they are used; any measurement they add is off until the
learner turns it on, by one switch. The journal's bullet adds that the monitoring service's copy
of DeckStreak's lines is kept 14 days too. The privacy manifest and the store's privacy details
stay as they are until the switch ships; then they declare product interaction, usage and
diagnostic data for analytics only, not linked to the learner and not tracking, and
`privacy.json` gains the switch's category with consent as its basis and the switch as its
withdrawal.

Chosen against:
- Declaring the switch's data in the manifest now: rejected because nothing is collected yet, and
  a label must describe what the app does.
- Saying nothing until the switch ships: rejected because the request counts start with the pager,
  and the page should say what the server counts when it starts.

### D9. Where the pager is declared, and the route

`docs/monitoring/pager.json` (`deckstreak.pager.v1`) declares the route, the log store's window,
what ships, the four pages, the channels and the request counters; `scripts/tests` holds it to the
tree. `deploy/slo.json` keeps its schema and changes values only: every alert's `route` and
`alerting.channel` become `pager`, the route the declaration names, and its note says so.

Chosen against:
- The pager inside `deploy/slo.json`: rejected because its schema refuses a key it does not know.
- A declaration under `deploy/`: rejected because the deploy installs what that directory holds,
  and nothing on the host reads this file.
- The route left as `journal`: rejected because the page no longer ends in the journal.

### D10. One delivery

The pager, the request counters and the switch's records land as one delivery: SPEC-585, this
ADR, the appended amendments to SPEC-031, SPEC-396 and ADR-410, one schematic, the declaration,
`deploy/slo.json`, the privacy page, two changed test modules and one new one, a rows band, the
red-first record and one changelog fragment. Its tests read files and execute no deploy script, so
each is seen red before the push and the delivery is pushed once.

Chosen against:
- Two deliveries, the pager first and the switch's records after: rejected because both change
  the privacy page, the counters belong to both, and one pull request at a time through the same
  queue doubles the land.
- The switch's code in this delivery: rejected because the apps' switch, the collector and their
  labels are each their own delivery, and none is needed for the pager.

### D11. FORMAL, decided by surface

NOT APPLICABLE for TLA+ and Lean. The delivery changes data and documents; the pages are matched
by the monitoring service, outside the tree, and no code of DeckStreak's gains a state, an
interleaving or a pure function. `formal/tla/SecondRoute` keeps its disposition: the script it
covers is unchanged, so its covers stay.

Chosen against:
- A TLA+ model of the pager's pages: rejected because the actor that reads the lines and decides
  is the monitoring service, which no model in this repository can bind to code.
- Retiring `formal/tla/SecondRoute` now: rejected because its script stays until its retirement
  (#1103).

## Superseded statements

Each record's text is kept; the row says what now holds.

| record | what it states for the install | what holds now |
|---|---|---|
| SPEC-031 (R2, R3), ADR-031, ADR-010 | every alert is routed to `telegram` (as ADR-601 leaves it, `journal`), and the template pages the owner | every alert is routed to `pager`; the pager reads the template's line off the host (D1, D9; SPEC-031's appended amendment) |
| SPEC-066, ADR-067, SPEC-061 | the alert template's own failure pages no one until a second route exists | `an-alert-instance-failed` tells it (D3) |
| SPEC-396, ADR-410, ADR-032 (its second-route amendment) | a second route, with its own unit, credentials and receiver, tells the owner of a failed or silent alert sender | the pager's alert-instance and absence pages (D3); the script, model, rows and tests stay until #1103 (the appended amendments) |
| SPEC-041, SPEC-354, SPEC-395, ADR-365, ADR-409, ADR-032 (the template's row) | the alert template posts to a chat service, makes one HTTPS request, or reaches off the host | the template sends nothing (ADR-601 D4); the pager reads its line, and its sandbox stays as ADR-601 decided |
| `docs/schematics/alert-and-slo-path.md` | a failing unit or a burning budget ends in one Telegram page, and the second route reports to a receiver | `docs/schematics/the-pager-the-request-counters-and-the-usage-switch.md` |

## Consequences

- Good, because every page the template writes reaches a person off the host with no credential
  on the host and no address DeckStreak must keep.
- Good, because the alert sender's failure and every kind of silence are heard by the same
  monitoring service, from records the sender does not write.
- Good, because the server's traffic is counted without reading anything from a device, and a
  count can never name a person.
- Good, because the switch's rules are settled before any app can measure anything.
- Bad, because the pager depends on a log shipper on the host and on the monitoring service's
  policies, which no repository test proves; the hand-run verify does (SPEC-585 section 8).
- Bad, because a page now waits on the shipper and the monitoring service's evaluation, minutes
  rather than seconds.
- Bad, because the observability pack's alert-route row reads this install as a broken alert path
  until the pack learns a channel other than Telegram.
- Bad, because the tree keeps the second route's script, model and tests with no unit that runs
  them, until their retirement.

## Confirmation

SPEC-585's acceptance criteria, each with its test and its row: the SLO declaration's route and
channel, the declaration's route, pages, window, shipped lines, channels and counters, and the
privacy page's statements; and the host's hand-run verify of each page (SPEC-585 section 8).

## What would make this wrong

- The service manager does not record each oneshot's successful run at the level the shipper
  reads: the absence pages would fire at once, the hand-run verify shows it, and D3's heartbeat
  line returns.
- The monitoring service cannot hold an absence window of 26 hours: the late-backup page becomes a
  daily count below one, decided when the policy is made and recorded here by an amendment.
- The log shipper does not fit inside the host's share: the pager then needs another reader of
  the journal, decided in its own ADR.
- A learner other than the owner arrives before the switch ships: the apps still send nothing,
  and nothing here changes.

## More Information

- ADR-601 and SPEC-582 (the Telegram-free install this starts from); ADR-410 and SPEC-396
  (superseded); ADR-031 and SPEC-031 (amended); ADR-032, ADR-067; SPEC-585.
- `docs/monitoring/pager.json`; `docs/schematics/the-pager-the-request-counters-and-the-usage-switch.md`.
