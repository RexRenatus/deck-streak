# SPEC-395: each loopback-only service is allowed only the peers it uses: its unit allows loopback alone and denies every other address, and a census over every listen setting holds the lists

- **Issue:** #679, the follow-up to #678. **Context(s):** none (the deploy templates and their
  tests, not a bounded context).
- **Decided by:** ADR-409 (D1 to D5). It keeps ADR-365 D1's rule for the identity endpoint (SPEC-354
  R1), which a deny of every address covers, and ADR-351 D6's lists for the sync server (SPEC-340
  R8), which it leaves as they are.
- **Status:** this pull request delivers R1 to R4 (section 3) and the rows of section 8.
- **Read at:** DeckStreak `dev` ce2bb195. Every `path:line` below is read there.

## 1. The problem, measured

Issue #679 asks that each service that listens on loopback alone be allowed exactly the peers it uses
and denied every other, its outbound peers measured and listed in its SPEC, and a test that holds the
list. Measured by `git grep -n` over `deploy/` and `scripts/tests/`:

- **The listeners.** Three settings in `deploy/deck-streak.env.example` name a listener, and no other
  setting matches `DECKSTREAK_*_LISTEN`: `DECKSTREAK_API_LISTEN` (line 34), `DECKSTREAK_MCP_LISTEN`
  (line 39) and `DECKSTREAK_SYNC_SERVER_LISTEN` (line 45). Each value was parsed as an address with a
  port and judged a loopback address; the values are not repeated here.
- **What each unit holds.**

  | unit | `IPAddressAllow=` | `IPAddressDeny=` | address families |
  |---|---|---|---|
  | `deploy/systemd/deck-streak-api.service` | none | `link-local` (line 55; SPEC-354 R1) | `AF_UNIX AF_INET AF_INET6` (line 52): "the loopback listener" (line 51) |
  | `deploy/systemd/deck-streak-mcp.service` | none | `link-local` (line 58; SPEC-354 R1) | `AF_UNIX AF_INET AF_INET6` (line 55): "the loopback listener" (line 54) |
  | `deploy/systemd/deck-streak-sync-server.service` | `localhost` (line 63) | `any` (line 64; SPEC-340 R8) | `AF_UNIX AF_INET` (line 60): "a loopback IPv4 address, and reaches nothing else" (line 59) |

- **The inbound peers each one uses**, every one of them on the host's loopback:
  - the API: the edge's API route, whose upstream is the API's own listen setting
    (`deploy/README.md:54-55`), and the deploy's readiness
    check on the same listener (`deploy/README.md:194-195`);
  - the MCP server: the owner's agent, which calls it on a loopback address
    (`deploy/systemd/deck-streak-mcp.service:1-2`, `deploy/systemd/deck-streak-mcp.service:6`,
    `deploy/README.md:16`);
  - the sync server: the edge's sync route, whose upstream is the sync server's own listen setting
    (`deploy/README.md:55-57`).
- **The outbound peers each one uses: none.**
  - The API's and the MCP server's IP families carry their loopback listener alone
    (`deploy/systemd/deck-streak-api.service:51`, `deploy/systemd/deck-streak-mcp.service:54`), and
    neither unit orders itself after the network being online, which each of the three units that
    reach off the host does (`deploy/systemd/deck-streak-bot.service:10-11`,
    `deploy/systemd/deck-streak-job@.service:12-13`, `deploy/systemd/deck-streak-alert@.service:12-13`).
  - The one outbound endpoint the example configuration names is the sync job's
    `DECKSTREAK_SYNC_ENDPOINT` (`deploy/deck-streak.env.example:51-53`); no setting names an endpoint
    the API or the MCP server calls.
  - The sync server reaches nothing else (`deploy/systemd/deck-streak-sync-server.service:59`).
  - Each unit's credentials arrive by `LoadCredential=` from the private rail's credential socket
    (`deploy/systemd/deck-streak-api.service:22-23`, `deploy/systemd/deck-streak-mcp.service:24-25`),
    which the service manager reads over a local socket: no IP peer.
- **The tests that read these lists.** `scripts/tests/test_deploy_templates.py` holds SPEC-354's
  identity census (lines 438-525, and its test at lines 2067-2155), whose table records the API and
  the MCP server under `link-local` (lines 456 and 458), and the sync server's lists (lines
  2255-2285). `scripts/tests/_units.py` admits `IPAddressAllow=localhost` and an `IPAddressDeny=` of
  `any` or `link-local` on a unit that loads a credential (lines 193-196 and 254-257). No test holds
  an allow list for a listener but the sync server.

## 2. Requirements

- **R1.** The API's and the MCP server's units each hold `IPAddressAllow=localhost` and
  `IPAddressDeny=any` in `[Service]`, after their address families, under a comment that names their
  peers, in place of the deny of the link-local range; the sync server's unit keeps the same pair.
  No other line of the three units changes.
- **R2.** A census reads each loopback listener's unit with its drop-ins and the reset rule, and
  refuses, naming the unit, one whose allow list is not `localhost` alone, one whose deny list is not
  `any` alone, and one that is absent. Planted cases on scratch copies prove each refusal.
- **R3.** The census's population is every listen setting in `deploy/deck-streak.env.example`: each
  `DECKSTREAK_*_LISTEN` setting maps to one unit in the census's table and is a loopback address. A
  setting the table lacks, a value that is not a loopback address and a table entry with no setting
  are refused by key and line, and no value is printed.
- **R4.** SPEC-354's identity census records the API and the MCP server under `any`, the deny word
  that now covers the endpoint's range, and its planted allow list on the API is re-anchored on the
  new deny line with its refusal unchanged. No test, planted case or row is removed.

## 3. Acceptance criteria of the loopback peer lists

| id | criterion | decided by |
|---|---|---|
| A1 | each listener's unit, read through its drop-ins, allows `localhost` alone and denies `any` alone; planted cases (the API allowing every address, the MCP server denying one range alone, a drop-in clearing the API's allow list, the sync server's allow list removed, the MCP server's unit absent) are each refused by the unit's name | `python3 -m unittest discover -s scripts/tests -p test_loopback_peers.py -k test_each_loopback_listener_allows_loopback_alone_and_denies_every_other_address` |
| A2 | the listen settings are exactly the census table's, each a loopback address; planted cases (a setting no unit holds, a wildcard address, a setting removed) are each refused by key and line | `python3 -m unittest discover -s scripts/tests -p test_loopback_peers.py -k test_the_listeners_are_every_listen_setting_and_each_listens_on_loopback` |
| A3 | SPEC-354's census records the API and the MCP server under `any`, and an allow of the link-local range planted beside the API's new deny is still refused | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_loopback_peers.py -k test_each_loopback_listener_allows_loopback_alone_and_denies_every_other_address
A2: python3 -m unittest discover -s scripts/tests -p test_loopback_peers.py -k test_the_listeners_are_every_listen_setting_and_each_listens_on_loopback
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint
```

A3's module executes deploy scripts, so CI runs it. The red-first record is
`docs/red-first/SPEC-395.md`.

## 4. File manifest

| path | context | change |
|---|---|---|
| `docs/specs/SPEC-395-each-loopback-only-service-is-allowed-only-the-peers-it-uses.md` | docs | added: this SPEC |
| `docs/decisions/ADR-409-each-loopback-listener-allows-loopback-alone-and-denies-every-other-address.md` | docs | added: D1 to D5 |
| `docs/schematics/deployment.md` | docs | a new last section, insert-only: each listener's peers |
| `docs/red-first/SPEC-395.md` | docs | added: A1 to A3 |
| `deploy/systemd/deck-streak-api.service` | deploy | the identity deny and its comment replaced by the pair and a comment naming its peers (R1) |
| `deploy/systemd/deck-streak-mcp.service` | deploy | the identity deny and its comment replaced by the pair and a comment naming its peers (R1) |
| `scripts/tests/test_loopback_peers.py` | tests | added: the census, A1 and A2 (R2, R3) |
| `scripts/tests/test_deploy_templates.py` | tests | two entries of the identity census's table and one planted case's anchor, in place, its line count unchanged (A3, R4) |
| `scripts/mutation-rows.d/S39500-S39599.json` | rows | added: section 8 |
| `scripts/mutation-rows.d/S35400-S35499.json` | rows | S35401 and S35403 re-anchored on the new deny line |
| `changelog.d/loopback-peers-395.md` | changelog | added |

## 5. What this does NOT do

- Applying the units on the host: the deploy of the release that carries them does it, and section 7
  is read there; the host step is #161's, ahead of #628's first deploy.
- Peer lists for the units that reach off the host (the bot and the job and alert templates): their
  peers are named hosts, not loopback, and each keeps its deny of the identity endpoint (#678).
- Rows for the API and the MCP server in the threat model's tables: that schematic is #653's.
- A listener's port: the lists match addresses, never ports, and each listener's bind is its own
  setting, which A2 holds to loopback (#679).

## 6. Risks

- A role calls an address off loopback that the units and the example configuration do not show:
  the deny drops it and the call fails. Detected before the change by the build's read of the roles'
  code at its cut, and after the deploy by section 7's journal read; the cure is a decision that
  names the peer.
- The edge or the readiness check moves off the host's loopback: the deny drops it. A2 refuses a
  listen setting off loopback, and section 7's readiness read detects the rest.
- A drop-in on the host widens or clears a list: A1 judges the repository's drop-ins, and section 7
  reads each unit's lists as the service manager holds them.
- SPEC-354's rows S35401 and S35403 lose their anchor when the deny line changes: this delivery
  re-anchors them, and CI's mutation job selects them by their target's path.

## 7. What the host shows after the deploy (read by a person, not by a repository test)

Read by the maintainer, on the host, after the deploy of the first release that carries this change;
the record is that read's hand-back.

| id | what holds | how it is read |
|---|---|---|
| H1 | each of the three units holds the loopback allow and the every-address deny | the service manager's view of each unit's two lists |
| H2 | no study, sync or chat flow changes (#679's third acceptance line) | the API answers its readiness route through the edge, a study session and an owner sync complete, and the bot answers |
| H3 | the MCP server answers its agent where the owner has started it, and no unit's journal shows a refused connection since the deploy | a call from the agent, and each unit's journal since the deploy |

## 8. The mutation rows

S39500-S39599, in `scripts/mutation-rows.d/S39500-S39599.json`. Each anchor and mutant is copied
from the cured unit, and every row's killer is A1's test.

- The API's allow list cleared (S39500), its deny narrowed to the link-local range (S39501), and its
  allow widened to every address (S39502).
- The same three on the MCP server (S39503, S39504, S39505).

SPEC-354's rows S35401 and S35403 keep their stems, mutants, descriptions and killer, and their
anchor moves to the new deny line. A2 judges the example configuration and A3 judges SPEC-354's
table in test code, neither of which this delivery's units change, so no row mutates them. The sync
server's lists keep SPEC-340's rows.

## 9. References

- Issue #679 and its predecessor #678.
- ADR-409, SPEC-354 and ADR-365, SPEC-340 and ADR-351 D6.
- `docs/schematics/deployment.md`, its last section.
