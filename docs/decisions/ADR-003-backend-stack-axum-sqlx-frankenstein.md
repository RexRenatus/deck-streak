---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The backend stack: axum 0.8, tokio 1.53, sqlx 0.9, tracing, and frankenstein for the bot

## Context and Problem Statement

The owner chose "port it to rust" and approved a house stack (the stack-selection pack's golden
paths, owner-approved on the radar). DeckStreak needs an HTTPS API for the Mini App, a Telegram
bot, a SQLite store and structured logs, on one small host within a stated memory budget
(CHARTER 3).

## Decision Drivers

- The owner-approved radar: Rust-first, golden paths `backend-api`, `data`, `telegram-bot`, `ops`.
- AI-buildability (the rubric's heaviest weight): Context7 coverage, compile-time checks that catch a model's old-syntax traps.
- Runtime fit on a small VM and a memory budget.

## Considered Options (the alternatives it was chosen against)

- axum 0.8 + tokio ~1.53 + sqlx 0.9 + tracing 0.1, and frankenstein for the Bot API — chosen: the golden path; sqlx checks queries against the migrations at compile time; frankenstein tracks the current Bot API.
- rusqlite instead of sqlx — rejected because its queries are strings checked at run time, and the golden path pins sqlx; it stays available only behind an ADR where sqlx cannot express a pragma.
- teloxide — rejected because the radar holds it (stalled at an older Bot API), so `no-hold-items` would refuse it.
- actix-web or Rocket — rejected because they are off the golden path and would need a deviation ADR scoring them on the rubric, with no measured gain.

## Decision Outcome

Chosen option: axum 0.8, tokio pinned `~1.53`, sqlx 0.9 with its offline query cache committed,
tracing 0.1 with tracing-subscriber 0.3 emitting JSON to journald, tower-http's layers (trace,
timeout, request id, sensitive headers, catch panic, body limit, concurrency limit), thiserror in
libraries and anyhow only in the binary, and frankenstein as the Bot API client. Each external
crate is admitted to `[workspace.dependencies]` by the wave that first uses it; `stack.json`
marks it `planned` until then.

### Consequences

- Good, because the golden path needs no deviation ADR and the stack probe holds the pins.
- Good, because axum 0.8's route syntax traps are caught by a router-building test and the rust-service probe.
- Bad, because sqlx's compile-time checks need a committed `.sqlx/` cache refreshed with every query change.

### Confirmation

The stack-selection pack's rows (`pinned-majors`, `no-hold-items`, `stack-matches-detected`) and the rust-service pack's rows run in `scripts/check.sh`.

## What would make this wrong

- frankenstein falls behind the Bot API the product needs (the radar's monthly refresh records it).
- sqlx 0.9 cannot express a pragma or a migration the ledger-sqlite practice requires, measured in the W0 database delivery.

## More Information

The stack-selection pack and `radar.json` (vendored under `.packs/`); the rust-service and observability packs.

Amendment (2026-09-28): one passage stating the host's size, in the context, was redacted under the
public-prose rule (ADR-059).
