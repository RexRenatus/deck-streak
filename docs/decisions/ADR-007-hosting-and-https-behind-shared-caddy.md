---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Hosting and HTTPS: the existing Caddy on the VM fronts the Mini App and its API

## Context and Problem Statement

A Mini App must be served over HTTPS, and so must the API it calls. The VM already runs Caddy on
ports 80 and 443 with automatic certificates, for another service on the same host. There is no
managed DNS zone; the host's static address has a free wildcard-DNS name that already holds a
certificate. The landing page needs a real domain for search.

## Decision Drivers

- No new firewall rule and no new public port (an anti-goal: no opened inbound ports).
- The shared Caddy is shared infrastructure: every change to it is an owner gate.
- The API should share the Mini App's origin so no CORS policy is needed.

## Considered Options (the alternatives it was chosen against)

- One new Caddy site block for the Mini App's host name, serving the static build with an SPA fallback and proxying `/api/*` to the API on a loopback port — chosen: zero new ports, automatic HTTPS, one origin, and the other service's block untouched.
- A separate public port for the API — rejected because it needs a firewall rule and a second certificate, and splits the origin.
- A tunnel service in front of the VM — rejected because the anti-goals forbid tunnels and opened inbound ports for the product.
- A managed platform (Cloud Run) — rejected because the service needs the local collection copy and the vault replica on the VM's disk.

## Decision Outcome

Chosen option: the API and the bot's webhook (if chosen over long polling in W0) listen on
loopback only. Caddy gets one new site block for the Mini App's host name: static files from the
current release's web build with `try_files {path} {path}.html /index.html`, `/api/*` proxied to
the API, the security headers the web-security pack reads (a CSP whose `frame-ancestors` admits
`https://web.telegram.org`, HSTS, nosniff, a referrer policy), `X-Robots-Tag: noindex`, and a
`robots.txt` that disallows everything. The host name defaults to a subdomain of the static
address's wildcard-DNS name at no cost; an owned domain is an owner decision (OWNER-SETUP), and
the landing page (ADR-014) needs one. The template is `deploy/caddy/deck-streak.caddy`,
parameterised by environment variables; the concrete host is private configuration.

### Consequences

- Good, because nothing new is exposed on the host.
- Bad, because a Caddy reload affects the co-hosted service; the first change is an owner gate and each later one is announced.

### Confirmation

The web-security header rows on the box against the Caddy snippet; cyber-pipeline's black-box stage after the owner authorizes the live host.

## What would make this wrong

- The owner buys a domain: the Mini App moves to its subdomain, a one-line change of private configuration and a Caddy block.
- The co-hosted service needs Caddy options that conflict with this block.

## More Information

docs/schematics/deployment.md; OWNER-SETUP.md; ADR-010; ADR-014.
