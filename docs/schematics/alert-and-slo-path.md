# Schematic: from a failing unit or a burning budget to one Telegram page

Kind: data flow, and the episode state machine the evaluator and the memory watch share. Read at
DeckStreak `main` e05dfa5 (ADR-010, `docs/schematics/deployment.md`, the observability pack's
`slo.template.json`, `alert@.template.service`, `alert-telegram.template.sh`,
`memory-watch.template.sh`) and redrawn at SPEC-031's delivery. Decided by ADR-031; built by
SPEC-031 over SPEC-032's units.

```mermaid
flowchart LR
  api[deck-streak-api.service] -->|TraceLayer response event at INFO: route, status, latency| journal[(journald)]
  roles[every role of deckstreakd] -->|a panic: the kernel's hook, one ERROR event through the redacting writer| journal
  slotimer[deck-streak-slo.timer, every 5 min] --> eval[slo-evaluate.py: long and short windows per alert]
  journal -->|the API's response events, read once over the longest window| eval
  eval -->|both windows past the burn, first time this episode| fail1[exit 1]
  mwtimer[deck-streak-memory-watch.timer, every minute] --> watch[memory-watch.sh: memory.events of every deck-streak cgroup]
  watch -->|new oom_kill or max| fail2[exit 1]
  watch -->|new high, or crossing 90 percent of memory.max| journal
  units[api, bot, job units: exit non-zero, a watchdog or an OOM kill] --> fail3[unit failed]
  fail1 & fail2 & fail3 -->|"OnFailure=deck-streak-alert@%n.service"| alert[alert-telegram.sh]
  journal -->|the failed run's last five error lines| alert
  creds[$CREDENTIALS_DIRECTORY: telegram-bot-token, owner-user-id] --> alert
  alert -->|URL and fields through curl --config on stdin| tg((Telegram sendMessage))
```

| page source | fires when | pages how often |
|---|---|---|
| a unit's own failure | the process exits non-zero, a watchdog timeout, an OOM kill | at each failure, including each one an automatic restart follows (systemd 254 and later start `OnFailure=` units on that failed state); a crash loop ends at the start limit |
| the SLO evaluator | both windows of an alert exceed its burn rate | once per burn episode (`$STATE_DIRECTORY`) |
| the memory watch | a new `oom_kill` or `max` event in a unit's `memory.events` | once per event (`$STATE_DIRECTORY`) |
| a scheduled job's transition | SPEC-027's paging transitions | once per episode |

## The alert template unit

`deck-streak-alert@.service` is the one alert path. Its instance is the failed unit's full name, so
a job instance's failure makes an instance whose name holds a second `@`, which systemd accepts:
`%i` is everything after the first `@` (systemd.unit(5)). It names no `OnFailure=` itself: a page
that fails must not start a page about the page. Its own refusal of an empty credential (SPEC-066
R3) is one line at error priority naming the credential and exit 1, before any request, so the
instance stays failed, in `systemctl --failed` and the journal; a page about it needs a route that
does not depend on the alert sender (#285). It reads no settings file and writes nothing: it holds
only its two credentials, the journal group that lets it quote the failed run's lines, and the
network.

```mermaid
sequenceDiagram
  participant systemd
  participant alert as alert-telegram.sh
  participant journal as journalctl
  participant curl
  systemd->>alert: %i, MONITOR_UNIT, MONITOR_SERVICE_RESULT, MONITOR_INVOCATION_ID
  alert->>alert: read telegram-bot-token and owner-user-id from $CREDENTIALS_DIRECTORY
  opt a credential is empty (SPEC-066 R3)
    alert-->>systemd: one line at priority 3 naming it, exit 1: the instance stays failed, no request
  end
  alert->>journal: that run's error lines (the unit's, when no run is named)
  journal-->>alert: at most the last five
  alert->>alert: the text: unit, result, lines#59; at most 3500 bytes, cut at a character boundary
  alert->>curl: --config - on stdin: url, chat_id, text (never argv)
  curl->>curl: POST sendMessage, three retries
```

## The episode, remembered in `$STATE_DIRECTORY`

The evaluator keeps the set of alerts burning at its last run; the memory watch keeps each unit's
counters with its cgroup's identity. A page is the edge from cool to burning, never the level.

```mermaid
stateDiagram-v2
  [*] --> cool
  cool --> burning: both windows past the burn: print at priority 3, exit 1, pages
  burning --> burning: still past it: print at priority 4, exit 0, pages no one
  burning --> cool: either window below it: the episode ends
  cool --> cool: nothing to say
```

The evaluator's own failure (a journal it cannot read, a declaration it cannot parse) is an episode
of the same machine under its own key, so a broken evaluator pages once rather than every five
minutes. The memory watch's first sight of a unit is its baseline; a cgroup made since its last run
(a restart, or a oneshot's next run) counts every event it holds as new.

## The second route (SPEC-396)

Kind: component and data flow, with the state machine of one run. Read at DeckStreak `dev`
`164ac206`, where the alert template names no `OnFailure=` (SPEC-066 R3) and its own failure is
recorded where the service manager and the journal show it; the route that tells the owner is
#285's (the section above). Decided by ADR-410; built by SPEC-396. The alert sender is unchanged;
this section adds a route with its own unit, its own process, its own two credentials and its own
far end.

```mermaid
flowchart LR
  subgraph first [the first route: the alert sender]
    units[every service and scheduled job] -->|"OnFailure=deck-streak-alert@%n.service"| atpl["deck-streak-alert@.service"]
    atpl --> ascript[the alert script]
    acreds[credentials: the bot token and the owner's id] --> ascript
  end
  subgraph second [the second route]
    rtimer[deck-streak-second-route.timer, every ten minutes] --> runit[deck-streak-second-route.service]
    runit --> rscript[second-route.sh]
    rcreds[credentials: second-route-check-in and second-route-report] --> rscript
    rstate[(STATE_DIRECTORY: the reported keys and the episode)] -->|"read at each run"| rscript
    rscript -->|"written only after a delivered report"| rstate
  end
  atpl -.->|"a failed instance stays failed and listed"| mgr[(the service manager)]
  mgr -->|"list-units: the failed alert instances, show: each one's invocation id, list-unit-files: the template's state"| rscript
  ascript -->|"a page"| owner((the owner))
  rscript -->|"a report: each new failed, absent or unreadable key, once"| recv((the receiver, off the host))
  rscript -->|"a check-in: the read was readable and every new key was told"| recv
  recv -->|"on a report, and on check-ins missing past the grace"| owner
  runit -->|"its own failure, once per episode: OnFailure=deck-streak-alert@%n.service"| atpl
```

| failure | the first route | the second route |
|---|---|---|
| a monitored unit fails | pages it | does not repeat it |
| the alert sender refuses an empty credential (SPEC-066 R3) | its instance stays failed | reports `failed <instance> <invocation id>` once |
| the alert sender's request is refused or unanswered after its retries | its instance stays failed | reports the same key once |
| the alert sender runs past its timeout, or is killed at its memory ceiling | its instance stays failed | reports the same key once |
| the socket does not deliver an alert credential | the start fails, the instance stays failed | reports the same key once |
| a later failure under a name already reported | its instance fails again | a new invocation id, so a new key, reported once |
| the alert template absent, masked or in another state | `OnFailure=` starts nothing | reports `template <state>` once |
| the service manager's state unreadable | not involved | reports `unreadable` once and withholds every check-in while it lasts |
| the second route's own failure: a credential empty or not https, a request not delivered | pages it once per episode | withholds the check-in |
| the host, its service manager or its network down, or both routes down | silent | check-ins stop, and the receiver tells the owner after its grace |

| route | process | credential roles it holds | far end |
|---|---|---|---|
| the alert sender | `deck-streak-alert@.service`, the alert script | the bot token, the owner's id | the owner's chat |
| the second route | `deck-streak-second-route.service`, `second-route.sh` | `second-route-check-in`, `second-route-report` | the receiver, off the host |

No credential id is in both rows, and no process: the second route never runs the alert script, and
asks the service manager only to list units, list unit files and show an invocation id. Each
credential reaches its own unit from the credential socket at every start (ADR-038), and the service
manager keeps each unit's credentials invisible to every other unit.

```mermaid
stateDiagram-v2
  [*] --> credentials
  credentials --> episode: a credential empty, or not an https address
  credentials --> read: both credentials hold an https address
  read --> report: a key not yet told
  read --> checkin: no new key, and the read was readable
  read --> withheld: no new key, and the read was unreadable
  report --> record: delivered
  report --> episode: not delivered
  record --> checkin: the read was readable
  record --> withheld: the read was unreadable
  checkin --> [*]: delivered, and the episode ends
  checkin --> episode: not delivered
  withheld --> [*]: exit 0, and the receiver's grace runs
  episode --> [*]: the episode's first run exits 1 and pages, a later run exits 0
```

A failed alert instance that a later instance of the same name replaces with a delivered page,
before the second route reads it, is told by that page: it names the same failed unit. The model
`formal/tla/SecondRoute` holds the order of these steps against a service manager that fails and
replaces alert instances between them (SPEC-396 §7).
