# Schematic: every pack judged on the box

Kind: component, then flow. Decided by ADR-069 and ADR-056; built by SPEC-056. The public gate runs
DeckStreak's own stages; the maintainer's box run judges every pack, the methodology probes, the
proxy-client scan and the owned data's drift, and posts one verdict on the pull request.

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
  packs --> boxed["the box section: each pack with its catalog's verb,<br/>expected reds and pending, by issue"]
  boxed --> probes[the sdd, ddd and tdd probes, and the proxy-client scan, from the checkout]
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
