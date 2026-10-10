---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Each loopback listener allows loopback alone and denies every other address in its own unit, and one census over every listen setting holds the lists

## Context and Problem Statement

Issue #679, the follow-up to #678, asks that each service that listens on loopback alone be allowed
exactly the peers it uses and denied every other, with its outbound peers measured and listed in its
SPEC, and a test that holds the list. SPEC-395 section 1 measures it at DeckStreak `dev` ce2bb195:

- three settings in `deploy/deck-streak.env.example` name a listener, each a loopback address with a
  port: `DECKSTREAK_API_LISTEN` (line 34), `DECKSTREAK_MCP_LISTEN` (line 39) and
  `DECKSTREAK_SYNC_SERVER_LISTEN` (line 45);
- every caller of those listeners sits on the host's loopback: the edge's API and sync routes, the
  deploy's readiness check, and the owner's agent;
- none of the three calls out: their IP families carry their listener alone, and none orders itself
  after the network being online, as the three units that reach off the host do;
- the sync server's unit holds `IPAddressAllow=localhost` and `IPAddressDeny=any` (ADR-351 D6,
  SPEC-340 R8), and the API's and the MCP server's units each hold `IPAddressDeny=link-local`
  (ADR-365 D1, SPEC-354 R1).

## Decision Drivers

- The issue's words: exactly the peers each service uses, declared in its unit, held by a test.
- Every listener's peers are on loopback, so the declaration needs no value the private
  configuration fills.
- The service manager reads a unit with its drop-ins and resets a list on an empty assignment, so a
  census must read the lists the same way.
- `scripts/tests/test_deploy_templates.py` executes deploy scripts, so it runs in CI alone, and other
  documents cite its lines by number.
- Each decision names what it was chosen against.

## Considered Options (the alternatives each decision was chosen against)

### D1. The population: the three loopback listeners

Chosen: the API, the MCP server and the sync server, each named by its `DECKSTREAK_*_LISTEN` setting,
because those are every listener the example configuration names and each one is a loopback address.
Their peers are SPEC-395 section 1's: inbound, callers on loopback; outbound, none.

**Chosen against:**

- The issue's two services alone: the sync server also listens on loopback alone, and a census that
  left it out would pass while its lists went missing.
- Every unit that opens an IP socket: the bot and the job and alert templates reach off the host by
  name, which an address list cannot hold without a private value, and #679 asks for the
  loopback-only services.

### D2. The mechanism: one pair of address lists in each unit

Chosen: every listener's unit holds `IPAddressAllow=localhost` and `IPAddressDeny=any` in
`[Service]`, after its address families, with a comment that names its peers. In the API's and the
MCP server's units the pair replaces the deny of the link-local range, which `any` covers, so
ADR-365 D1's rule keeps holding. The service manager matches the allow list first and the deny list
next, so each service exchanges packets with loopback addresses of both families and with nothing
else.

**Chosen against:**

- A host firewall rule: the API, the MCP server and the bot run as one service user
  (`deck-streak-api.service:17`, `deck-streak-mcp.service:18`, `deck-streak-bot.service:20`), so a
  rule by user cannot tell them apart, and the rule lives outside the repository, where no test
  reads it.
- A socket bind restriction alone (`SocketBindAllow=`, `SocketBindDeny=`): it bounds the ports a
  service binds, never the peers it exchanges packets with.
- A private network namespace (`PrivateNetwork=yes`): the listener would bind inside the service's
  own namespace, where the edge on the host's loopback could not reach it
  (`deck-streak-api.service:14-15`).
- Narrower address families: the listener needs the IP families, which cannot tell a loopback peer
  from any other.
- The exact listen address in the allow list: it is a value the private configuration fills, while
  `localhost` is the word both existing censuses already admit (`scripts/tests/_units.py:254-257`,
  `scripts/tests/test_deploy_templates.py:438-444`).
- A different pair per unit: one pair for all three keeps one census rule and one row shape.
- Keeping the link-local deny beside `any`: the deny of every address already covers that range, and
  with a second word SPEC-354's cleared-deny row (S35401) would survive, since `any` would still
  cover the range.
- Nothing (the identity deny alone): it denies one range, and the issue asks for exactly the peers
  each service uses.

### D3. The check: a census of every listener in its own module

Chosen: a new module, `scripts/tests/test_loopback_peers.py`, that imports only the unit reader
(`scripts/tests/_units.py`), the examined-count helper (`scripts/tests/_support.py`) and the
standard library, and runs no subprocess. A1 reads each listener's unit through its drop-ins, with
the reset rule, and refuses a unit whose allow list is not `localhost` alone, whose deny list is not
`any` alone, or that is absent, naming the unit; planted cases on scratch copies prove each refusal.
A2 closes the population: the listen settings in the example configuration are exactly the census
table's, each a loopback address, refused by key and line and never by value. SPEC-354's identity
census records the API and the MCP server under `any`, and its planted API allow list is re-anchored
on the new deny line (A3). Each red is read in CI at the red commit, pushed alone. Rows S39500 to
S39505 mutate the two units' lists; SPEC-354's rows S35401 and S35403 are re-anchored on the new
deny line.

**Chosen against:**

- A new class in `scripts/tests/test_deploy_templates.py`: that module executes deploy scripts, so
  its tests run in CI alone, and the threat model cites its lines by number.
- Widening SPEC-354's identity census: its rule is the identity endpoint's range, and it admits a
  deny of that range alone, which is narrower than a listener's pair.
- Narrowing the credential census's values (`_units.PAGING_VALUES`): they bound every unit that
  loads a credential, the bot's deny of the link-local range among them.
- A row in the durable-services lint: it carries no peer-list row, and it runs off this
  repository's CI.

### D4. FORMAL, decided by surface: not applicable

Chosen: no model and no proof, because two static lines per unit and a census over files add no
actor, no shared state and no step between a check and an act, and no formal entry covers a file
this delivery edits.

**Chosen against:**

- A state-machine model of the address filter: its allow-before-deny order is the service
  manager's own, a third-party engine, and the delivery adds no interleaving to model.
- A machine-checked proof of the census: it is a total function over a unit's lines, which its
  planted cases already hold, and the precedence such a proof would state is not this delivery's
  code.

### D5. Drift and the host

Chosen: the edit to `scripts/tests/test_deploy_templates.py` keeps its line count and touches no
line another document cites; the threat model's tables gain no row; the units reach the host with
the deploy of the release that carries them, and a person reads them there (SPEC-395 section 7).

**Chosen against:**

- Adding the API's and the MCP server's rows to the threat model's tables here: that schematic is
  #653's, and work in flight edits it.
- Applying the units on the host in this delivery: a pull request changes templates, and the host
  takes them with a release's deploy.
- Re-deriving the threat model's citations: no cited line moves, so there is nothing to re-derive.

## Decision Outcome

Chosen: D1 to D5, because each listener's callers sit on loopback and it calls nothing, so an allow
of loopback and a deny of every other address is exactly its peers, and one census whose population
is every listen setting holds each unit's lists.

### Consequences

- Good, because each listener exchanges packets with loopback alone, its declaration sits in its own
  unit, and a drop-in that widens or clears a list is refused by A1.
- Good, because a new listen setting with no unit in the census's table is refused by A2.
- Bad, because the lists match addresses, never ports: any loopback peer may reach a listener, and
  the listener's own bind and request guard (SPEC-119 for the MCP server) hold the rest.
- Bad, because a role that later calls off the host needs its lists changed by a decision that
  supersedes this one.

### Confirmation

A1 to A3 (SPEC-395 section 3), each red at the red commit and green after the fix, read in CI; rows
S39500 to S39505 and the re-anchored S35401 and S35403 killed in CI's mutation job; on the host,
SPEC-395 section 7.

## What would make this wrong

- A role is found calling an address off loopback, by a read of its code or a refused connection in
  its journal: that peer is then measured, named in SPEC-395, and allowed by a decision of its own.
- The edge or the deploy's readiness check moves off the host's loopback.
- The service manager stops applying a unit's address lists to its traffic.

## More Information

- Issue #679; SPEC-395 (`docs/specs/SPEC-395-each-loopback-only-service-is-allowed-only-the-peers-it-uses.md`).
- SPEC-354 and ADR-365 (#678), whose rule D2 keeps; SPEC-340 and ADR-351 D6, whose sync-server lists
  D1 takes as the shape.
- The deployment schematic's last section, "Each loopback listener's peers (SPEC-395, ADR-409)".
