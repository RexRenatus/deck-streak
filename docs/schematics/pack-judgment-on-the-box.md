# Schematic: every pack judged on the box

Kind: component, then flow. Decided by ADR-069 and ADR-056; built by SPEC-056. The public gate runs
DeckStreak's own stages; the maintainer's box run judges every pack, the methodology probes, the
proxy-client and apiKeyHelper scans and the owned data's drift, and posts one verdict on the pull
request. The third table is how a criterion whose test the removal took away is retired, and the
fourth how the durable lint's advisory departures are judged.

```mermaid
flowchart LR
  subgraph public[public: CI and any builder]
    gate[scripts/check.sh] --> stages["fmt, clippy, test, test-engine, doctest,<br/>audit-rust, web, audit-web, python, scrub, secrets"]
    stages --> jobs["CI jobs: rust, engine, web, hygiene"]
    scrub[scripts/public-scrub.py] -->|reads| rules[(scripts/scrub-rules/)]
    vault[deck-streak-vault] -->|compiles in| data[(crates/vault/data/)]
  end
  subgraph box[the maintainer's box]
    private[(the private wiring file)] --> driver[scripts/box-packs.sh]
    checkout[(the private checkout, at the pin)] --> driver
    runner[the runner, built from it] --> driver
  end
  driver -->|judges the committed tree at REV| verdicts["one line per pack, probe, scan and owned file"]
  driver -. "--post-status" .-> status["commit status box/packs: success, failure or error"]
  data -. "drift check, field by field" .-> driver
  rules -. "drift check, field by field" .-> driver
```

```mermaid
flowchart TD
  start[box-packs.sh --rev REV] --> env{PACKS_WIRING, PACKS_CHECKOUT and PACKS_RUNNER set and usable?}
  env -- no --> void[VOID by name, exit 2]
  env -- yes --> pin{the checkout's HEAD is the file's pin?}
  pin -- no --> void
  pin -- yes --> export[export the committed tree at REV into a scratch directory]
  export --> issues[read every issue the box section names with gh]
  issues -- unreadable --> void
  issues --> packs["the packs section: each pack through the runner, --scope tree,<br/>judged enforced, pending or deferred, with excluded and deferred rows"]
  packs --> waivers["durable-services: the lint from the checkout, every advisory departure<br/>waived in its unit or waiting on an open issue"]
  waivers --> boxed["the box section: each pack with its catalog's verb,<br/>expected reds and pending, by issue"]
  boxed --> probes[the sdd, ddd and tdd probes, and the proxy-client and apiKeyHelper scans, from the checkout]
  probes --> drift{each owned file equal to its source, over the fields it keeps?}
  drift -- a source is missing --> void
  drift --> summary{any FAIL?}
  summary -- yes --> failed[BOX PACKS FAILED, exit 1]
  summary -- no --> ok[BOX PACKS OK, exit 0]
```

| verdict | exit | `box/packs` state |
|---|---|---|
| BOX PACKS OK | 0 | `success` |
| BOX PACKS FAILED | 1 | `failure` |
| VOID, with its reason | 2 | `error` |

| the apiKeyHelper scan finds | the tree holds a settings file where the old gate step looked | its line |
|---|---|---|
| no settings file | no | `pending #N` with the private file's issue, or `FAIL` (VOID) without one |
| no settings file | yes | `FAIL` (VOID), naming the file |
| settings files, none carrying the shape | either | `ok`, or `FAIL` (stale) while an issue is still named |
| a finding, or a file it cannot read | either | `FAIL` |

| a retired criterion, insert-only (SPEC-056 R14) | where |
|---|---|
| its id struck, `~~A3~~` | the SPEC's criteria table |
| its command in a `retired` fence, splitting the `acceptance` fence where it stood (closed empty before a first line) | the SPEC's section 3 |
| its red, green or not-red lines in a `retired` fence | its red-first record |
| why its subject is gone, and what judges it now | the SPEC's dated amendment section, and SPEC-056 section 7 |

| an advisory departure the lint reports, by unit and reason (SPEC-056 R15) | the pack reads |
|---|---|
| waived in its unit, with a why of more than five words | counted as waived |
| waived with a why of five words or fewer | `FAIL`, a thin why |
| not waived, and waiting on an issue the private file names, which is open | counted as waiting |
| neither waived nor waiting | `FAIL`, naming the unit and the reason |
| a waiver or a waiting entry that matches no departure, or waits on a closed issue | `FAIL`, stale |
