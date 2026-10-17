# SPEC-354: every service that does not use the host's identity is denied the host's identity endpoint, and the services that use it are a closed list

- **Issue:** #678 (SEC-01's host review, item 2), one of the preconditions of #628's first
  deploy; the host step is #161's. **Context(s):** none (the deploy templates and their tests, not
  a bounded context).
- **Decided by:** ADR-365 (D1 to D6), beside ADR-351 D6 (the sync server's peers, SPEC-340 R8),
  which it leaves as it is.
- **Status:** this pull request delivers R1 to R4 (section 3) and the rows of section 8.
  **Mutation band:** S35400-S35499. **Model:** none; no formal model reads a unit this
  changes.

Public text: the requirement and its acceptance only. The review's detail stays in its private
record.

## 1. The problem, measured

Read at dev `02a8e458`: each service's `[Service]` lines by `grep -n -E
'IPAddress|RestrictAddressFamilies|SystemCallArchitectures|ExecStart' deploy/systemd/*.service`,
then the script or binary each command runs.

| service | uses the host's identity | evidence | its lines at the base | this delivery |
|---|---|---|---|---|
| `deck-streak-api.service` | no | `deckstreakd api` (:16); no crate under `crates/` names the endpoint or carries a client for it | families `AF_UNIX AF_INET AF_INET6` (:52); no IP list | `IPAddressDeny=link-local` |
| `deck-streak-bot.service` | no | `deckstreakd bot` (:19); same crate reading | families `AF_UNIX AF_INET AF_INET6` (:61); no IP list | `IPAddressDeny=link-local` |
| `deck-streak-mcp.service` | no | `deckstreakd mcp` (:17); same crate reading | families `AF_UNIX AF_INET AF_INET6` (:55); no IP list | `IPAddressDeny=link-local` |
| `deck-streak-job@.service` | no | `deckstreakd job %i` (:17); its jobs are `crates/coordination/src/jobs.rs`'s; same crate reading | families `AF_UNIX AF_INET AF_INET6` (:51); no IP list | `IPAddressDeny=link-local` |
| `deck-streak-alert@.service` | no | its alert script (:17), which posts to the chat service alone | families `AF_UNIX AF_INET AF_INET6` (:48); no IP list | `IPAddressDeny=link-local` |
| `deck-streak-sync-server.service` | no | `deploy/scripts/sync-server.sh` (:21) | families `AF_UNIX AF_INET` (:60); `IPAddressAllow=localhost` (:63); `IPAddressDeny=any` (:64) | none: it already denies every peer but loopback (SPEC-340 R8) |
| `deck-streak-backup.service` | no | `backup.py` (:15) | families `AF_UNIX` (:44); native (:50) | none: it opens no IP socket |
| `deck-streak-sync-snapshot.service` | no | `backup.py --sync-window` (:26) | families `AF_UNIX` (:53); native (:59) | none: it opens no IP socket |
| `deck-streak-sync-restore-drill.service` | no | `restore-drill.sh --part sync` (:20) | families `AF_UNIX` (:50); native (:56) | none: it opens no IP socket |
| `deck-streak-slo.service` | no | `slo-evaluate.py` (:12) | families `AF_UNIX` (:44); native (:50) | none: it opens no IP socket |
| `deck-streak-memory-watch.service` | no | `memory-watch.sh` (:13) | families `AF_UNIX` (:43); native (:49) | none: it opens no IP socket |
| `deck-streak-litestream.service` | yes | `litestream replicate` (:19) over `deploy/litestream.yml`, whose replica is the bucket (:24); SPEC-064 section 6, "The replica's identity is the host's" | families `AF_UNIX AF_INET AF_INET6` (:55) | on the closed list; a comment says so |
| `deck-streak-sync-archive.service` | yes | `backup.py --sync-archive` (:22), whose copy command sends the sealed archive to the bucket (`deploy/scripts/backup.py`:291); `docs/runbooks/sync-server-cutover.md`:52 | families `AF_UNIX AF_INET AF_INET6` (:54) | on the closed list; a comment says so |
| `deck-streak-restore-drill.service` | yes | `restore-drill.sh` (:13), which runs `litestream restore` from the replicator's bucket (`deploy/scripts/restore-drill.sh`:70); the unit's own comments (:18, :43) | families `AF_UNIX AF_INET AF_INET6` (:44) | on the closed list; a comment says so |

Fourteen services. Three use the host's identity to reach their bucket. Five open an IP socket and
hold no deny list that covers the host's identity endpoint, and at the base A1's census refuses
exactly those five by name (`First list contains 5 additional elements`). Five open no IP socket.
One, the sync server, already denies every peer but loopback.

## 2. Requirements

R1. Every service under `deploy/systemd/` that is not on R2's list is refused, by the kernel, every
    route to the host's identity endpoint, in one of two ways:
    - its `RestrictAddressFamilies=` is an allow list that names no IP family (`AF_INET`,
      `AF_INET6`) and it runs on the native ABI alone (`SystemCallArchitectures=native`); or
    - its `IPAddressDeny=` holds `link-local` or `any`, and its `IPAddressAllow=` holds nothing but
      `localhost`.
    The API, the bot, the MCP server, the job template and the alert template each gain
    `IPAddressDeny=link-local`. The sync server keeps its lists as SPEC-340 R8 set them.
R2. The services that use the host's identity are a closed list, held in the census test and named
    by file: the replicator (`deck-streak-litestream.service`), the sync archive's upload
    (`deck-streak-sync-archive.service`) and the restore drill
    (`deck-streak-restore-drill.service`). An entry whose unit file the tree does not ship is
    refused. No key, drop-in or comment in a unit exempts it.
R3. The census enumerates every `deploy/systemd/*.service`, reading its drop-ins and a template's
    instance drop-ins after systemd's reset rule. It prints its examined count and refuses zero. It
    refuses, by name, a service that takes neither of R1's ways and is not on R2's list, and it
    pins the way each service takes: the deny word that covers the endpoint, "opens no IP socket",
    or exempt.
R4. The credential census admits `IPAddressDeny=link-local` beside `any` on a service that loads a
    credential, and admits the `IPAddressDeny=` key on the alert template. It still refuses a deny
    of any other range on such a service, and any `IPAddressAllow=` on the alert template.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every shipped service takes the way to the endpoint the census's table names, none but the closed list's reaches it, and seven planted ways to it (a deny removed, a deny of another range, a wider allow on a denied unit, a wider allow on the sync server, an IP family on a no-IP unit, a family deny list, an instance drop-in's reset) are each refused by name | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint` |
| A2 | every entry of the closed list is a shipped unit, and a copy of the units that lacks one is refused by that entry's name | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_an_exempt_entry_whose_unit_is_absent_is_refused` |
| A3 | the credential census admits the `link-local` deny on a paging unit and on the alert template, and refuses a deny of another range on a paging unit and an allow list on the alert template | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_credential_census_admits_the_identity_deny_and_no_wider_peer_list` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_an_exempt_entry_whose_unit_is_absent_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_credential_census_admits_the_identity_deny_and_no_wider_peer_list
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-354-identity-endpoint-denied.md` | docs | new: this SPEC |
| `docs/decisions/ADR-365-identity-endpoint-denied.md` | docs | new: D1 to D6 |
| `docs/red-first/SPEC-354.md` | docs | new: the red-first record |
| `changelog.d/identity-endpoint-denied-354.md` | docs | new: one Security bullet |
| `deploy/systemd/deck-streak-api.service` | deploy | `IPAddressDeny=link-local` and its comment, after `RestrictAddressFamilies=` |
| `deploy/systemd/deck-streak-bot.service` | deploy | the same |
| `deploy/systemd/deck-streak-mcp.service` | deploy | the same |
| `deploy/systemd/deck-streak-job@.service` | deploy | the same |
| `deploy/systemd/deck-streak-alert@.service` | deploy | the same |
| `deploy/systemd/deck-streak-litestream.service` | deploy | a comment naming its place on the closed list; no key |
| `deploy/systemd/deck-streak-sync-archive.service` | deploy | the same |
| `deploy/systemd/deck-streak-restore-drill.service` | deploy | the same |
| `scripts/tests/test_deploy_templates.py` | tests | the census's helpers and table; A1 and A2 in `TheServicesRunTheirRoles`; A3 in `ARefusedCredentialFailsItsUnitAndPages`; `import shutil`; the sync server's value-table pin reads both admitted words |
| `scripts/tests/_units.py` | tests | `ALERT_KEYS` gains `IPAddressDeny`; `PAGING_VALUES` admits `link-local` beside `any` for `IPAddressDeny`; their comments |
| `scripts/mutation-rows.d/S35400-S35499.json` | tests | new: S35401 to S35408 |

## 5. What this does NOT do

- It runs nothing on a host. Reading each denied unit's effective lists, the slices above them, the
  kernel's enforcement and the exempt units' first copies is a recorded step of #161, on the
  owner's go.
- It narrows no service's peers beyond the host's identity endpoint. The API and the MCP server
  listen on loopback and could deny every other peer; #679 holds that.
- It changes no host firewall, slice, user, credential or privilege; a host change is a step of
  #161.
- It does not change the deploy rail's effective check, which reads no IP list; #161's recorded
  step reads the lists on the host instead.
- It adds no deny list to a service that opens no IP socket; R1's family way covers them, and A1's
  census refuses one that gains an IP family (#678).

## 6. Risks

- The kernel does not apply the IP lists on the host; systemd documents that they then have no
  effect, and asks that they not be relied on alone. Detected by #161's recorded step (c), a
  transient run under the service user refused at the endpoint, before the first start.
- A denied service resolves names through a resolver that is not on loopback, and its lookups
  fail. Detected by the same step (c): the same run resolves and connects to a public name a denied
  service uses, before the first start.
- The endpoint is not in the range the deny list names on the host. Detected by step (c).
- A slice above the services carries an allow list; an allow wins over a deny, and a slice's lists
  combine with its units'. Detected by step (b), before the first start.
- A later service opens an IP socket and holds no deny: A1's census refuses it by name.
- A later service needs the host's identity: it joins the list only by an edit to the census's
  closed list, which review reads, and A2 refuses an entry whose unit has gone.

## 7. Delivered by the next pull requests

None.

## 8. The mutation rows

S35400-S35499, in `scripts/mutation-rows.d/S35400-S35499.json`. Each row's anchor and
mutant are copied from the cured file, and every row's killer is A1's test.

- A denied service's deny cleared (A1): the API (S35401), the bot (S35402), the MCP server
  (S35403), the job template (S35404) and the alert template (S35405).
- A deny of a range that does not hold the endpoint (A1): the bot's (S35406).
- A service that opened no IP socket opens one (A1): the SLO evaluator (S35407).
- The sync server admits the link-local range (A1): S35408.

A2 and A3 judge the census's own tables, which live in test code; no row mutates a test.

## 9. References

SPEC-340 (R8), SPEC-062, SPEC-064, ADR-351 (D6); `deploy/litestream.yml`,
`deploy/scripts/backup.py`, `deploy/scripts/restore-drill.sh`; systemd.resource-control(5)
(`IPAddressAllow=`, `IPAddressDeny=`), systemd.exec(5) (`RestrictAddressFamilies=`,
`SystemCallArchitectures=`); #628, #161, #678, #679.
