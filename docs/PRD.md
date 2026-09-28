# Product Requirements: deck-streak

The campaign's full requirements, parity matrix and wave plan are
[SPEC-001](specs/SPEC-001-campaign-prd-parity-and-waves.md). This page is the product summary.

## Problem

The owner studies languages and law in Anki every day. Their existing gamification service works
through a chat bot only, its charts are drawn on the server, and its flagship idea (a short
reading before each day's new cards) has never reached them. They want the whole game, the second
brain's daily study companions and an AI mentor per subject in one Telegram Mini App.

## Users

One owner per deployment: a single learner who runs their own Anki sync server and a small VM.
The repository is public so others can run their own deployment.

## Goals

- Full parity with the predecessor's 121 built features, proved by the parity oracle.
- Daily pre-study readings in the Mini App, written by a named mentor per subject.
- One notification policy across the bot and the Mini App.
- An AI agent for the digest's coaching and the second-brain duties, fail-closed.

## Non-goals

- Multi-tenant hosting: each deployment serves one owner.
- AnkiWeb access: DeckStreak reads a self-hosted sync server only.
- Any countdown, exam date or forward-looking timeline.

## Success metrics

| metric | target | how it is measured |
|---|---|---|
| parity rows closed with evidence | every built row of SPEC-001 | closed issues carrying an evidence comment |
| readings delivered on study days with new cards | 95 percent of such days | the readings lane's run records |
| notification SLO | 99 percent of routed messages delivered or deliberately withheld | the router's decision ledger |

## Requirements

SPEC-001 lists every requirement, and the per-wave SPECs carry each one's acceptance criteria.

## Release plan

- W0 foundation, W1 the readings, W2 the first deploy, then the game (W3), curriculum and
  insights (W4), engagement (W5), the second brain and the AI duties (W6), surfaces and polish
  (W7), and the data migration and cutover to `v1.0.0` (W8).
- Each wave boundary that leaves `dev` green is a release: a pull request from `dev` into `main`.

## Open questions

- The owned domain for the landing page and the Mini App (OWNER-SETUP).
- The owner's decisions on the predecessor's inert features, one issue each.
