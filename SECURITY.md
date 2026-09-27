# Security Policy

## Supported Versions

| version | supported |
|---|---|
| the latest release tagged on `main` | yes |
| the `dev` branch | yes, for reports against unreleased code |
| older releases | no: upgrade to the latest release |

## Reporting a Vulnerability

Report a vulnerability privately through GitHub's private vulnerability reporting:
<https://github.com/RexRenatus/deck-streak/security/advisories/new>.

Never open a public issue, discussion or pull request for a vulnerability. Include what you found,
how to reproduce it, and the impact you expect.

## What happens next

- You get an acknowledgement within 7 days.
- A fix, or a reasoned decision not to fix, follows within 90 days, and you are told which.
- The advisory is published when a fixed release exists, crediting you unless you ask otherwise.

## Dependency advisories

Dependabot alerts are on and its automated security fixes stay off: those would open against
`main`, which only a release pull request from `dev` may change (ADR-036). The maintainer triages
each alert. A fixable one becomes a pull request into `dev`, and ships with the next release, or
at once as a patch release when it cannot wait. A dismissed alert carries its reason. Version
updates keep arriving as pull requests into `dev`.

## Handling credentials

DeckStreak never stores a credential in the repository, an environment variable, a command line, a
log line or a database row. Secrets live in a secret manager and reach each service as a systemd
credential at start. If you find a credential in this repository or its history, report it
privately as above; it will be rotated first and removed second.

## Scope

In scope: this repository's code, its CI workflows and its deploy templates. Out of scope: the
Anki sync server a deployment reads (report those to its operator), Telegram's platform, and a
denial of service that needs more traffic than the documented rate limits allow.
