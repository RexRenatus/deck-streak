---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The AI route uses the rail's loopback forward, and the repository ships no tunnel

## Context and Problem Statement

ADR-015 chose a reverse forward for the agent's route: the proxy listens on loopback on the
maintainer's machine, and a forward opened from that machine makes it appear on the host's own
loopback, so the host holds no credential to the maintainer's machine. The planned SPEC-063 (#43)
went further and had this repository specify the forward: a supervised tunnel unit (R1) and a
dedicated accepting account with its restricted key line and its own server drop-in (R2). The
private rail already provides a loopback forward on the host, and a forward's listen address is
held by one forward at a time. The forward is host configuration the private rail owns, and the
repository's tests cannot judge it (ADR-069). Who specifies the forward, and how does the agent
reach the proxy?

This amends ADR-015's "a reverse SSH tunnel opened from the maintainer's machine" only as far as who
specifies the tunnel: the rail does, not this repository.

## Decision Drivers

- A listen address is held by one forward at a time: a second remote forward on it fails its
  `ExitOnForwardFailure` check and its unit restarts in a loop.
- The proxy tells its clients apart by device key, so the agent needs its own key and needs no
  forward of its own.
- The forward, its account and its server-side settings are host configuration, which the box run
  judges on the maintainer's box only (ADR-069); a public test cannot judge them.
- The host holds no credential to the maintainer's machine, and the forward listens on loopback
  only (ADR-015).
- The route is optional and `Absent` by default (ADR-054), and the device key is a credential from
  the socket (ADR-038).

## Considered Options (the alternatives it was chosen against)

- The agent reaches the proxy through the loopback forward the rail already provides; the repository ships no tunnel unit, no tunnel account, no sshd drop-in and no second forward, and plans none; the endpoint is a setting and the key a credential from the socket — chosen: it adds no listener, no account and no server setting, the proxy still distinguishes the agent by its own device key, and nothing the repository's tests cannot judge is specified in the repository.
- (A) SPEC-063 R1 and R2 as planned: a tunnel unit and a dedicated accepting account, specified by this repository — rejected because a second remote forward on a listen address the rail's forward already holds fails its `ExitOnForwardFailure` check and restarts in a loop, and because the forward is host configuration the rail owns and the repository's tests cannot judge (ADR-069).
- (B) A second forward on a distinct loopback port — rejected because it adds a second listener, a second accepting account, a second server drop-in and a second egress rule for nothing the proxy needs, since the proxy tells clients apart by device key.

## Decision Outcome

Chosen option: "the rail's loopback forward, and no tunnel in the repository", because it reaches the
proxy with the fewest moving parts and leaves each thing with the party that can judge it.

- **The forward is the rail's.** The agent reaches the proxy at the host's loopback address through
  the forward the private rail provides. The repository ships no tunnel unit, no tunnel account's
  `authorized_keys` line, no sshd drop-in and no second forward. The rail never opens a second
  forward on a listen address it already holds.
- **The endpoint is a setting.** `deploy/optional/ai-route/ai-route.env.example` names the route
  setting and a loopback base URL with neutral values (SPEC-063 R2).
- **The key is a credential from the socket.** The device key is the owner's, added to the proxy's
  roster by the maintainer with the owner's go, and reaches the readings unit as a credential
  (ADR-038); SPEC-063 R5 is unchanged.
- **A test keeps it so.** A planted tunnel unit, key line, server drop-in or second forward under
  `deploy/` or `agent/` is refused (SPEC-063 A7).

### Consequences

- Good, because no second listener, account or server setting exists to fail, restart in a loop or
  drift from the rail's.
- Good, because the repository specifies only what its own tests can judge.
- Good, because SPEC-063's private steps shrink: no tunnel account, no server drop-in.
- Bad, because enabling the route needs the owner's device key (#162) and a host-side loopback
  admission for the route's unit, which is the rail's act, so the route cannot be enabled from the
  repository alone.
- Bad, because the agent depends on the rail's forward being up; a run with it down ends
  `proxy_unreachable`, alerted once (SPEC-063 R11).
- This repository plans no proxy restart (SPEC-063 section 6).

### Confirmation

SPEC-063 A7 (nothing under `deploy/` or `agent/` ships one of the four shapes, and a planted one of
each is refused) with its mutation row S06301; the box run judges the rail's forward on the
maintainer's box (ADR-069); the live proof (SPEC-063 R11).

## More Information

Issue #43. SPEC-063 (planned, amended by this decision), ADR-015 (amended as far as who specifies the
forward), ADR-038, ADR-054, ADR-061, ADR-063, ADR-069; SPEC-043 and SPEC-053, which SPEC-063 is
built after.
