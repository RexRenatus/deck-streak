---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Both surfaces decide a vault card through one use case, and the package is a bot document to the owner's chat

## Context and Problem Statement

The owner decides each vault card candidate (#65, SPEC-150): approve, edit then approve, or reject.
The W9 plan binds the tap to the owner alone (ADR-006), and nothing unapproved may reach a package.
DeckStreak has two owner surfaces, the bot and the Mini App, in separate processes over one database,
and every message goes through the one router or is a command reply its census names (SPEC-041).
Where does the owner's tap happen, how do two surfaces avoid deciding one card twice, and how does
the package reach the owner and no one else?

## Decision Drivers

- The owner's tap is owner-only on every surface (ADR-006, CHARTER 14).
- A decision from either surface follows the same rules, and two at once apply one.
- The package reaches the owner's own chat and nothing public, and it is never logged.
- No second delivery path around the router's census (SPEC-041), and no push the owner did not ask
  for.

## Considered Options (the alternatives it was chosen against)

- One coordination use case both surfaces call, guarded in the store, and the package sent by the bot as a document into the owner's chat: chosen, because it keeps one rule set for the decision and one owner-gated path for the file.
  The bot's buttons and the Mini App's actions call `coordination::vault_cards::decide`; the store's
  conditional update and its trigger make a second decision of one revision `not_pending`. The
  package is a command reply to /vaultpack, sent with the transport's `send_document` into the chat
  the owner gate admitted.
- Each surface writing the decision itself: rejected because two copies of the rules drift, and the
  census could no longer prove that only one code path decides.
- A lock in each process's memory: rejected because the bot and the API are separate processes, so
  a lock in one never sees the other; the database's own write is the only guard both share.
- A package download from the API in the Mini App: rejected because a webview download needs a
  route that serves the owner's file over the session, a new surface for a private file, where the
  bot already sends the data-rights export as a document.
- Telegram's Mini App file download: rejected because it fetches from a URL, which would have to
  carry the owner's credential or expose the file to anyone holding the link.
- A link to the file in storage: rejected because a link can be forwarded and outlives the request,
  and the file would then exist outside the owner's chat.
- A router occasion that pushes the package when cards are approved: rejected because the owner
  asked for nothing, and a requested answer is its command's reply (ADR-113).

## Decision Outcome

Chosen option: "one coordination use case both surfaces call, and the package as a bot document to
the owner's chat", because both surfaces then share one rule set and one guard, and the file travels
only where the owner already receives the export.

- **The tap.** In the bot, the buttons Approve, Edit and Reject under each pending card (`va:`, `ve:`
  and `vr:` with the revision id), behind the owner gate; in the Mini App, the same three actions,
  behind the owner's session. Both call `decide`.
- **The guard.** The decision is one conditional update of a `pending` revision inside `Db::write`,
  and the migration's trigger allows only the moves SPEC-150 R11 names (a `rejected` or `edited`
  revision is final), so the second of two decisions answers `not_pending` whichever surface or
  process sent it.
- **The package.** /vaultpack only, in the owner's chat, as `vault-cards.apkg`, with a caption of
  counts; the Mini App says where the package comes from and offers no download.

### Consequences

- Good, because the owner can decide on either surface, and a card decided on one is shown as
  decided on the other.
- Good, because the file never has a URL, a route or a log line, and reaches only the chat the gate
  admits.
- Bad, because the owner who works in the Mini App switches to the bot for the file. The screen says
  so, and a download there would need its own decision.

### Confirmation

SPEC-150's A16 to A18 and A27 to A32 (the decisions, the race, one use case, the bot's buttons and
the owner-only routes), SPEC-151's A12, A13 and A16 (one caller chain, the document in the owner's
chat, nothing for anyone else), and SPEC-152's A2 to A4.

## What would make this wrong

- A third surface for the owner: it would call the same use case, and its reply path would need its
  own census entry.
- A package larger than Telegram accepts from a bot: the reply refuses it (SPEC-151's
  `package_too_large`), and a split or another path would need a new ADR.

## More Information

SPEC-150, SPEC-151, SPEC-152, SPEC-041 (the router and its census), SPEC-026 (the owner gate),
SPEC-024 (the owner's session), ADR-006, ADR-113, and the schematics
`docs/schematics/vault-card-candidate-lifecycle.md` and `docs/schematics/vault-card-package-delivery.md`.
