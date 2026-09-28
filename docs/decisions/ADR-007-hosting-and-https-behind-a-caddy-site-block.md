---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Hosting and HTTPS: a Caddy site block fronts the Mini App and its API

## Context and Problem Statement

A Mini App must be served over HTTPS, and so must the API it calls. DeckStreak runs on one small
host shared with other services, within a stated budget (CHARTER 3), behind Caddy, which obtains
and renews its certificates automatically. The Mini App's host name is private configuration, and
the landing page needs a real domain for search.

## Decision Drivers

- No new firewall rule and no new public port (an anti-goal: no opened inbound ports).
- Every change to Caddy's configuration is an owner gate.
- The API should share the Mini App's origin so no CORS policy is needed.

## Considered Options (the alternatives it was chosen against)

- One new Caddy site block for the Mini App's host name, serving the static build with an SPA fallback and proxying `/api/*` to the API on a loopback port — chosen: zero new ports, automatic HTTPS, one origin, and one block to review and to reverse.
- A separate public port for the API — rejected because it needs a firewall rule and a second certificate, and splits the origin.
- A tunnel service in front of the VM — rejected because the anti-goals forbid tunnels and opened inbound ports for the product.
- A managed platform (Cloud Run) — rejected because the service needs a local disk for its collection copy and the files it writes.

## Decision Outcome

Chosen option: the API and the bot's webhook (if chosen over long polling in W0) listen on
loopback only. Caddy gets one new site block for the Mini App's host name: static files from the
current release's web build with `try_files {path} {path}.html /index.html`, `/api/*` proxied to
the API, the security headers the web-security pack reads (a CSP whose `frame-ancestors` admits
`https://web.telegram.org`, HSTS, nosniff, a referrer policy), `X-Robots-Tag: noindex`, and a
`robots.txt` that disallows everything. An owned domain is an owner decision (OWNER-SETUP), and the
landing page (ADR-014) needs one. The template is `deploy/caddy/deck-streak.caddy`, parameterised
by environment variables; the concrete host name is private configuration.

### Consequences

- Good, because nothing new is exposed on the host.
- Bad, because every change to Caddy's configuration waits for the owner: the first is an owner gate, and each later one is announced.

### Confirmation

The web-security header rows on the box against the Caddy snippet; cyber-pipeline's black-box stage after the owner authorizes the live host.

## What would make this wrong

- The owner buys a domain: the Mini App moves to its subdomain, a one-line change of private configuration and a Caddy block.
- The Mini App needs a Caddy option that a site block cannot carry.

## More Information

docs/schematics/deployment.md; OWNER-SETUP.md; ADR-010; ADR-014.

Amendment (2026-09-28): passages throughout this record that described co-hosted infrastructure
and the host's name were redacted under the public-prose rule (ADR-059); the title and the file
name changed with them. This is a security redaction, the one kind of edit an accepted document's
text may take; the originals remain in the repository's history.
