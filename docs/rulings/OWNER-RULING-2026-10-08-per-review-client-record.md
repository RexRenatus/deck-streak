The owner rules that every review in the owner's review log that DeckStreak reads also carries which client made it, and amends SPEC-334 R14 (its MET-01 sentence only) so that the new interface's learning outcomes may be split by client. The record is one field per review, naming DeckStreak's web client, DeckStreak's iPhone and iPad client, a stock client, or unknown. It is a category of DeckStreak's own database, never written to the collection, and it is exported and erased as every other category is. Besides the export, the erase and the backups, only R14's measurement and the owner's own view of the record read it. An unknown client is never read as any other value.

# OWNER RULING 2026-10-08: the per-review client record

## What was held

SPEC-334 R14 (`docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:83-86`) measures whether
the new interface worked "by learning outcomes from the review log only (days studied, true retention, minutes per
retained card), with no new telemetry".

DeckStreak is the owner's main study client, and stock clients still sync (ANK-08). The owner wants each review to say
which client made it. A per-review client field is a new signal, which R14's "no new telemetry" forbids.

## What it rules

- **The record.** Every review in the owner's review log that DeckStreak reads carries one client field, with exactly
  four values: DeckStreak web, DeckStreak iPhone and iPad, stock, and unknown. It starts with the first review
  recorded after the build lands; earlier reviews read unknown. A review is recorded as stock only on positive
  evidence that a stock client made it. A review with no such evidence, including a DeckStreak review whose record
  never arrived, reads unknown, and unknown is never read as any other value.
- **How a client is learned.** That is the build's design, but nothing is kept to learn it beyond the field itself:
  any signal used to tell clients apart is read in passing and never stored, and neither the sync server nor the edge
  keeps anything new for it. If a client holds the record on the device until it syncs, the privacy page's list of
  what the device keeps names it.
- **Where it lives.** It is a category of DeckStreak's own database, never written to the collection, so it adds no
  write to the collection (ADR-301). The export includes it and the erase deletes it, as for every other category,
  though the review it describes stays in the collection. It leaves DeckStreak's service only as every other category
  of its database does (its export and its backups), and by no other route: never to a model, and to no party beyond
  those that already carry the export and the backups. PRIVACY.md's table and privacy.json list it as a category
  before any build writes it.
- **Who reads it.** Besides the export, the erase and the backups that every category passes through, only R14's
  measurement and the owner's own view of the record read it. No score, XP, streak, nudge or AI duty reads it.
- **R14, amended (MET-01 only).** R14's first sentence is unchanged. Its second now reads: "Whether the new interface
  worked is measured by learning outcomes from the review log and the per-review client record only (days studied,
  true retention, minutes per retained card), and the measurement may split those outcomes by client, with no new
  telemetry beyond that record." The delivery that builds the field edits SPEC-334 R14 to this text, and SPEC-334's
  Decided by line cites this ruling.
- **What stays.** Stock clients keep reaching the sync server under ANK-08 and SPEC-334 R9, and this ruling changes
  nothing about their sync. ADR-301 (its never-list, ladder and single writer), SPEC-334 R7 and every other category's
  export and erase are unchanged. Nothing else DeckStreak keeps or sends changes.

## What it was chosen against

- **Keeping R14 unchanged, and building the field only after the new interface has been measured.** DeckStreak being
  the owner's main study client would have no record in the reviews, and the new interface's first outcomes could
  never be split by client.
- **Building the field without amending R14, as attribution that no measurement reads.** The owner chose to let the
  measurement split its outcomes by client.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
