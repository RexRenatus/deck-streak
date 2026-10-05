---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Every service that does not use the host's identity is denied its endpoint by a per-unit deny list, and the services that use it are a closed list a test holds

## Context and Problem Statement

SEC-01's host review asks, as its item 2, that every DeckStreak service that does not need the
host's identity be refused, by the kernel, any route to the host's identity endpoint, and that the
services that do use it stay exempt by a closed list. SPEC-354 measured fourteen services:
three use the identity to reach their bucket (the replicator, the sync archive's upload and the
restore drill), five open no IP socket, one (the sync server) already denies every peer but
loopback (SPEC-340 R8, ADR-351 D6), and five open an IP socket with no deny list that covers the
endpoint. This record decides how those five are denied, how the others are judged, where the
closed list lives, and how the host confirms it.

## Decision Drivers

- Deploy-class and hand-run: nothing in this delivery runs on a host, and the install is a
  recorded step of #161 on the owner's go.
- No new privilege, rail, user, credential or unit kind.
- Public text carries no address: a symbolic name, never a literal.
- A miss is killed by a test that enumerates every service, prints its examined count and refuses
  zero.
- systemd.resource-control(5): a packet that matches an allow entry is granted before the deny
  list is read; the lists of the slices above a unit combine with its own; an empty assignment
  resets a list; `link-local` and `any` are symbolic names, and `any` holds the link-local range.
- systemd.exec(5): `RestrictAddressFamilies=` restricts socket(2), and is to be combined with
  `SystemCallArchitectures=native`.

## Considered Options (the alternatives it was chosen against)

D1, how a service that opens an IP socket is denied the endpoint:

- Chosen: `IPAddressDeny=link-local` in each of the five units, after its `RestrictAddressFamilies=`
  line, because it names the range that holds the endpoint by its symbolic name, keeps
  every other peer the services reach, and adds one line per unit that the census reads.
- A host firewall rule that matches by user: rejected because the replicator and the bot run as
  the same user, so a rule by user cannot tell an exempt service from a denied one, and a host
  firewall is a host change this delivery does not make (#161).
- A slice that denies every peer, with allow lists on the exempt services: rejected because it adds
  a unit kind the tree does not ship, its allow lists would have to name the endpoint, an allow
  wins over a deny, and the census would read every unit's lists through a parent.
- `IPAddressDeny=any` with `IPAddressAllow=localhost` on each of the five: rejected because the
  bot, the jobs and the alert template reach public services, and the lists filter by address
  alone, so an allow list would have to name those services' addresses. The API and the MCP server
  could take it; #679 holds that.
- `PrivateNetwork=yes`: rejected because the API and the MCP server serve the edge on the host's
  loopback, and the bot, the jobs and the alert template need public peers.
- An address literal in the deny list: rejected because public text carries no address, and the
  symbolic name covers the range in both IP families.

D2, a service that opens no IP socket:

- Compliant by its families: a `RestrictAddressFamilies=` allow list that names neither IP family,
  with `SystemCallArchitectures=native`, takes no deny list: chosen, because the kernel already
  refuses it every IP socket, and the census plants an IP family into one such unit to prove the
  way is judged, not assumed.
- A deny list on those five too: rejected because it changes five units for a route the kernel
  already refuses them, and it adds five credential-census values for no behaviour.
- A family deny list (`~...`) read as no IP socket: rejected because a deny list of families
  admits every family it does not name; the census reads one as a service that may open an IP
  socket.

D3, where the closed list lives:

- Chosen: A tuple in `scripts/tests/test_deploy_templates.py`, each entry a unit file name, and an entry
  whose unit the tree does not ship refused, because the list is closed in one place a
  reviewer reads, and a stale entry cannot outlive its unit.
- A key in each exempt unit (an `X-` key systemd ignores): rejected because a unit could exempt
  itself by adding a line, so the list would not be closed.
- A list file under `deploy/`: rejected because the host never reads it, so it is a second artifact
  with the test as its only reader.
- Exemption by user: rejected because a user does not decide whether a service uses the identity;
  the replicator and the bot share one.

D4, how the census reads a unit:

- Chosen: Through `scripts/tests/_units.py`, which reads a unit's drop-ins and a template's instance
  drop-ins and applies systemd's reset rule, because the tests that already judge the units
  read them this way, and a drop-in that clears a deny is a way the census must see.
- `systemctl show` or `systemd-analyze` over installed units in CI: rejected because it needs the
  units installed on a running systemd, which CI is not; the host reads them that way in #161's
  step instead (D6).
- A pattern over the raw unit files: rejected because it misses a drop-in's reset.

An allow entry beyond `localhost` refuses a unit, whatever its deny list holds, because an allow is
granted first.

D5, the credential census:

- Chosen: `PAGING_VALUES` admits `link-local` beside `any` for `IPAddressDeny=`, and `ALERT_KEYS` admits
  the `IPAddressDeny=` key, because the census bounds values per key, and A1's table pins
  each unit's word by name; the sync server's own lists stay pinned by SPEC-340's test.
- A per-unit value table: rejected because the census's tables are per key, and a second table
  would restate A1's.
- Leaving the census unchanged and the five without a deny: rejected because the credential census
  would then refuse R1's own lines.
- Dropping the value bound on `IPAddressDeny=`: rejected because it would admit a deny of a range
  that does not hold the endpoint.

D6, how the host confirms it:

- Chosen: A recorded step of #161, attached to the units' first start (SPEC-062's row), on the owner's go,
  because every host change in this project is a recorded, hand-run step, and the step
  reads what CI cannot (the effective lists, the slices, the kernel's enforcement).
- A read of the IP lists in the deploy rail's effective check: rejected because it changes the
  rail, which this delivery does not do.
- A check in the deploy script on every deploy: rejected because it runs on the host without the
  owner's go, as a pipe.

## Decision Outcome

The chosen option of each of D1 to D6. Five units gain one key each; three gain a comment; the
census's two tables admit the new word; the census, its controls and eight rows hold it.

### Consequences

- Good, because a service that never uses the host's identity cannot reach its endpoint, and a
  later service is refused by name until it does one of R1's two things or joins the closed list.
- Good, because no privilege, user, credential, rail or unit kind is added.
- Bad, because the IP lists depend on the kernel's support on the host; #161's step proves it
  before the first start, and a failure stops the deploy.
- Bad, because a denied service's name lookups depend on a loopback resolver; the same step proves
  a public name resolves and connects from a denied run.

### Confirmation

SPEC-354's acceptance tests A1 to A3, its rows in S35400-S35499, and the recorded step on
the host (#161).

## What would make this wrong

- The host's identity endpoint moves out of the link-local range: D1's word misses it, and #161's
  step (c) catches it before the first start.
- A service that is denied comes to need the identity: it fails to reach its bucket, and joins the
  closed list by an edit to the census, which review reads.
- systemd changes what `link-local` names: the step's transient run catches it.

## More Information

SPEC-354; SPEC-340 (R8) and ADR-351 (D6); SPEC-062; SPEC-064; systemd.resource-control(5);
systemd.exec(5); #628, #161, #678, #679.
