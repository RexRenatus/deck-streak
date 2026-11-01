---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-601: The install carries no Telegram surface before the Telegram code leaves the tree

## Context and Problem Statement

ADR-589 deprecated the Telegram Mini App and the Telegram bot: DeckStreak is the web app and the
iPhone and iPad app. It kept their code (`crates/bot/`, identity's launch-data port, the web app's
Telegram mapping) until one removal delivery takes it out after native push carries the router
(ADR-341, #640), and it named no step for the install in between.

The install, every file under `deploy/` that `deploy.sh` and the private rail put on a host, still
carries the Telegram surfaces at the base this decision was measured on:

- the bot's unit, `deploy/systemd/deck-streak-bot.service`, its rail-contract rows and its
  host-budget entry, and `deploy.sh`'s two restarts of it;
- the owner's `/sync` doorbell: the path unit of the sync job and the tmpfiles line of its request
  directory, which only the bot writes;
- the edge's framing policy, which admits Telegram's web client;
- the settings example's Mini App URL and doorbell path;
- three units outside the bot that load the bot's two credentials, `owner-user-id` and
  `telegram-bot-token`: the API (its owner gate over Telegram's launch data), the alert template
  (its sender pages on Telegram), and the held flush's instance (its router sends through the bot);
- the second route, which watches the alert sender and reports to two addresses off the host.

So an install without the bot's two credentials cannot start: the API refuses start without them,
and the deploy then switches back. What leaves the install now, in what order against the code's
removal, and what does the alert template run once its Telegram sender leaves?

## Decision Drivers

- The install carries only what DeckStreak is: the web app's API, the sync server, the jobs, the
  backups, and their observability.
- No surface may lose its owner gate while its code still runs (ADR-589). A gate that is off must
  refuse every launch, so nothing is reachable without one.
- A unit that fails still leaves one record of its failure that a later monitoring delivery can
  page from. No failure is silent.
- Records stay records: a landed SPEC, ADR or changelog fragment is kept, and a later record
  supersedes its statements.
- No new credential, host package or paid service enters with this delivery.
- The smallest change that installs: the code's removal keeps its own delivery.

## Considered Options (the alternatives each decision was chosen against)

Each decision below names the options it was chosen against, each reason on its first line.

## Decision Outcome

### D1. The order: the install drops the Telegram surfaces first, and the code leaves later

ADR-589's removal order is amended. The install drops every Telegram surface in this delivery; the
code stays in the tree, deprecated, until the removal delivery ADR-589 names, which still follows
native push carrying the router (#640).

Chosen against:
- One delivery that removes the install's surfaces and the code together: rejected because the
  code's removal waits on native push and reaches the web app, so the install would wait with it.
- A deploy-only removal (the bot unit, the doorbell, the framing origin and the bot restarts):
  rejected because the API, the alert template and the held flush would still load the bot's two
  credentials, so the install would still need them.
- Keep the install as shipped until the code leaves: rejected because the install would carry a
  deprecated bot, its credentials and a pager that depends on it.

### D2. What leaves the install, what changes, and what stays

Leaves:
- `deploy/systemd/deck-streak-bot.service`, its two rail-contract rows and its host-budget entry;
- the sync job's path unit (`deck-streak-job@sync`, path unit) and
  `deploy/tmpfiles.d/deck-streak-sync-request.conf`;
- the held flush instance's drop-in `20-bot-credentials.conf`, and with it the instance's drop-in
  directory;
- the second route's unit and timer, their two rail-contract rows and the unit's host-budget entry
  (D6);
- the two `LoadCredential=` lines of the API unit and of the alert template;
- `deploy.sh`'s restart of the bot after the API's readiness, and its restart of the bot in the
  switch back;
- Telegram's web client from the edge's `frame-ancestors`, which becomes `'none'`;
- `DECKSTREAK_SYNC_REQUEST_PATH` and `DECKSTREAK_MINI_APP_URL` from
  `deploy/deck-streak.env.example`;
- `deploy/scripts/alert-telegram.sh`, replaced by the journal sender (D4).

Changes:
- the alert template's `ExecStart=`, to the journal sender, and its comments;
- `deploy/slo.json`: every alert's `route` and the `alerting.channel` become `journal`;
- the deploy README's tables and runbook, and the owner setup's Telegram sections, which say the
  install carries no bot, no Mini App and no Telegram identity.

Stays:
- the alert template's name, `deck-streak-alert@.service`, and its sandbox lines as they are, so
  every `OnFailure=` line, `slo.json`'s `alerting.unit` and the second route's script stay true;
- the held flush's timer and its job in coordination's table (D5);
- `DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS` in the settings example, which the API still parses while
  the launch-data port is in the tree;
- the root `.env.example`, which documents the code's local runs, the bot role among them;
- the second route's script, its formal model and its script tests (D6);
- DeckStreak's share of the host and its processor count (D7).

Chosen against:
- Remove the bot's settings from the root `.env.example`: rejected because it documents the code
  that stays, and leaves with that code.
- Remove the launch-data age setting: rejected because the API still parses it, and the code's
  removal takes it out.
- Tighten the alert template's sandbox to no IP socket now: rejected because it would move the
  template's arm in the host-identity census and the row that holds its deny list, for a sender
  the monitoring delivery decides again.

### D3. The owner gate is off when neither Telegram credential is held

When the API's credentials hold neither `owner-user-id` nor `telegram-bot-token`, or the unit
passes no credentials directory at all, the API starts with Telegram off:

- `POST /api/session` and `POST /api/launch` answer 404 with the reason `telegram_off`, identity's
  refusal, written as `linking_off` is: the gate itself refuses, so a body that is not the
  handshake's JSON keeps its 401 `init_data_invalid`, and a well-formed one answers 404
  `telegram_off`;
- `POST /api/link/code` and `POST /api/link/redeem` answer 404 `telegram_off` after the
  state-change guard and the ceremony bound and before `linking_off`, as `linking_off` is judged
  before any session;
- the passkey routes and `/api/identities` answer `linking_off`: no owner is pinned for them;
- every owner route answers 401 `no_session`, since no session can open.

One credential held without the other refuses start by the missing one's id, as at the base. Both
held, the gate is the gate it was.

Chosen against:
- A placeholder owner behind an always-refusing key: rejected because an owner pin that names no
  one reads as a gate and could be taken for one.
- A route guard ahead of the body: rejected because it moves the handshake's lines that the threat
  model cites and the launch route's rows anchor on; the refusal inside the gate leaves both as
  they stand.
- Refuse start without the credentials, as at the base: rejected because the install carries none.
- Remove the routes: rejected because their code leaves with the launch-data port, in the removal
  delivery.

What this leaves for the sign-in line: SPEC-024's handshake stays in the code and is off on the
install; SPEC-359's link code is off with Telegram, and its passkey ceremonies stay, answering
`linking_off` until an owner pin that does not come from Telegram exists; SPEC-385's web passkey
screens are untouched; SPEC-403's launch validation answers `telegram_off`, which the page treats
as a refused launch, so it loads no Telegram script. The off constructor of the session routes'
state is the one seam where the apps' own owner sign-in supplies the owner; the sessions, the
cookie and the cross-site bound are unchanged.

### D4. The alert template's sender writes one line to the journal

`deploy/scripts/alert-journal.sh` replaces the Telegram sender. For the unit `%i` it writes one
line at error priority to the journal, under the alert instance's own identifier: the failed unit,
its result and exit status, and no line of the failed run (D9). It reads no credential, makes no
request and exits 0 once the line is written; a line it cannot write leaves the alert instance
failed, in `systemctl --failed`.

The options, the chosen one first:
1. The journal sender. Cost: no credential, no host package, no service. What it can page: nothing
   off the host by itself; it writes the one record a monitoring service off the host can match,
   by the alert instance's identifier at error priority. What the monitoring delivery changes: it
   adds the reader of that line and its alert, or replaces the sender, and keeps the template's
   name.
2. A log-shipping agent on the host with an alert on a monitoring service and an email channel:
   rejected for this delivery because it needs a host package and an alerting policy, which are
   host and cloud acts outside this repository; its in-repository half is option 1.
3. Mail from the host through a relay: rejected because it needs a new credential and a network
   path for the alert template.
4. A third-party mail service: rejected because it needs a vendor account and its data processing
   terms.
5. No sender until push alerts land: rejected because the template would either fail at every page
   or write nothing, and a failure would leave no stable record.
6. Wait for the monitoring platform's own design: rejected because the install would carry the
   Telegram sender until then.

No option sends to Telegram.

### D5. The held flush keeps what it holds, with no transport

The held flush's job and its timer stay. Its credentials drop-in leaves. When neither Telegram
credential is held, its router is the holding router with no transport: a flush delivers nothing,
and every held item stays held for the transport native push brings (#640).

Chosen against:
- The timer leaves the install, with the job's row in coordination's table and its calendar
  tests: rejected because the job table is the one schedule, the liveness check reads every job's
  fires, and native push needs the flush back.

### D6. The second route leaves the install now

The second route's unit and timer leave `deploy/systemd/`, with their rail-contract rows and the
unit's host-budget entry. Its script, its formal model and the tests that run the script stay in
the tree for the monitoring delivery to keep or retire.

Chosen against:
- It waits, installed, for the monitoring delivery: rejected because its two addresses off the host
  do not exist, the rail's pair check needs both, and a unit that cannot start pages at every fire.

### D7. DeckStreak's share of the host stays as it was decided

The share and its processor count stay as ADR-064 and ADR-347 decided. The bot's memory ceilings
and its processor quota are freed, not handed to another unit: the long-running units' ceilings
plus the largest job's now fit inside the share, and the daemons' quotas fit inside its
processors.

Chosen against:
- Shrink the share to the new sum: rejected because the share is a decision about the host's other
  tenants, which the hosting install owns.
- Give the freed share to another unit: rejected because no unit here asks for more.

### D8. The records

This ADR amends ADR-589's removal order by an appended section. An amendment is appended to each
record that a test or an acceptance fence this delivery changes cites: SPEC-024, SPEC-031,
SPEC-032, SPEC-059, SPEC-062, SPEC-396 and ADR-347. Every other statement that names a removed
unit, the Telegram sender, a removed credential pair or the Telegram framing origin as part of the
install is superseded by the table below, and its record's text is kept.

Chosen against:
- Amend every record the census names: rejected because ADR-589 superseded its statements by one
  table, and a hundred appended notes would say the same thing.
- Rewrite the records: rejected because records are history.

### Superseded statements

| record | what it states for the install | what holds now |
|---|---|---|
| SPEC-026, ADR-026 | the bot's unit runs its long-polling transport and owner gate | the unit is not installed; the code stays (D2) |
| SPEC-059, ADR-066 | the owner's `/sync` rings a path unit through a request directory | neither is installed; the sync job runs on its own timer (D2) |
| SPEC-041 (R7 as amended for #291), ADR-124, ADR-300 | the held flush sends through the bot, with the bot's pair in its own drop-in | the drop-in is not installed; the flush keeps what it holds (D5) |
| SPEC-031, ADR-031, ADR-010, ADR-038, SPEC-066, ADR-067 | the alert template pages the owner on Telegram with the bot's pair, and refuses an empty one | the journal sender reads no credential (D4) |
| SPEC-396, ADR-410 | the second route is installed beside the alert template | it is not installed; its script stays (D6) |
| SPEC-024, SPEC-025, ADR-006, ADR-024 | the API refuses start without the bot's pair | with neither held it starts with Telegram off (D3) |
| SPEC-032, SPEC-062, ADR-007, ADR-025, ADR-032 | the units table, the deploy's restart of the bot, and the edge admitting Telegram's frame | D2 |
| SPEC-028, SPEC-400, SPEC-403, ADR-414, ADR-417 | Telegram's web client may frame the app, and the launch route validates a launch | the edge admits no frame; the launch route answers `telegram_off` on the install (D2, D3) |
| SPEC-354, SPEC-395, ADR-409 | the bot is one of the units denied the host's identity endpoint | the bot is not installed; the planted ways move to the alert template |
| SPEC-333, ADR-064, ADR-347 | five daemons and the largest job fill the share, and five quotas divide its processors | four daemons and the largest job fit inside it (D7) |
| SPEC-083, SPEC-341, ADR-352 | name the bot's unit, the doorbell or the framing origin in passing | as the rows above |

### D9. The alert line quotes no line of the failed run

The journal sender's line names the failed unit, its result and its exit status and quotes no line
of the failed run, because error text in an alert can carry user data and the unit's own lines
already reach the shipped journal; a line that quoted the run's last error lines is rejected for
that reason.

### Consequences

- Good, because a host installs DeckStreak without any Telegram credential, and the rail's
  credential pairs fall from 16 to 6.
- Good, because the alert template's name, its `OnFailure=` callers and the SLO declaration's
  alert unit are unchanged, so the monitoring delivery changes one sender.
- Good, because the held items are kept, not dropped, for the transport native push brings.
- Bad, because no page reaches the owner off the host until the monitoring delivery lands: a
  failure is a line in the journal and an entry in `systemctl --failed`.
- Bad, because no web sign-in exists on the install until the apps' own owner sign-in lands; the
  sync server, the daily jobs and the backups run without one.
- Bad, because the held queue grows by what the router holds until a transport is joined; that
  transport's first flush abandons what outlived the policy's age and names it in its recap.
- Bad, because the tree keeps the bot's code, the launch-data port and the second route's script
  without a unit that runs them, until their own deliveries.

### Confirmation

SPEC-582's acceptance criteria, each with its test and its row: the API's three credential cases,
the routes' `telegram_off` answers, the held flush's kept items, the census that no shipped source
outside the bot names the Bot API, the units and credentials the install ships, the journal
sender, the SLO declaration's channel, the deploy's restarts, the edge's framing policy, the rail
contract and the host budget.

## What would make this wrong

- A hosting step needs a page off the host before the monitoring delivery lands; that delivery then
  comes first, and D4's sender is its starting point.
- The apps' own owner sign-in cannot pin the owner without Telegram; D3's off gate then holds until
  a ruling on the identity model.
- Native push cannot carry the router; the held items then wait on a ruling on the bot's
  transport, and D5 still keeps them.

## More Information

- ADR-589 (amended here), ADR-341, ADR-064, ADR-347; SPEC-582.
- `docs/schematics/the-install-carries-no-telegram-surface.md`.
