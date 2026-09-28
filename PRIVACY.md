# DeckStreak privacy policy

DeckStreak serves one owner: the person who runs it, whose Anki reviews it scores. It keeps only
what it needs to run that study, in its own database. This page says what that is, why, on which
lawful basis, for how long, and what an erase leaves behind. [privacy.json](privacy.json) is the same
inventory, in the form the repository's privacy checks read.

## What DeckStreak keeps

| category | what | purpose | lawful basis | retention |
|---|---|---|---|---|
| `sync-history` | the record of each sync of your collection: when it ran, what started it, its outcome and its attempts | running your daily sync, and telling you when it stops | contract | until account deletion |
| `ingest-state` | what the last recompute of your reviews saw, whether you asked for a rescore, and the base of the review window | recomputing your scores only when your collection or your settings changed | contract | until account deletion |
| `service-counters` | one counter of how many times your settings changed | telling the recompute that a setting changed | contract | until account deletion |

The lawful basis of each is the contract (GDPR Article 6(1)(b)): the service you run needs it. None
is kept for a fixed period. Each is kept until you erase it, which is how an account is deleted here.

Outside its database, DeckStreak holds a private copy of your collection, your own Anki data,
refreshed from your sync server; and, in memory only, your Telegram user id in a signed-in session.

## Your copy of your data, and erasing it

You export and erase on the host that runs DeckStreak:

- `deckstreakd data export` writes every category above as one JSON document, whose `schema` is
  `deckstreak.export.v1`, with one key per table holding its rows.
- `deckstreakd data erase --confirm ERASE` erases every category above in one transaction: each
  table's rows are deleted, and each one-row table is reset to its starting values. The pages the
  erase frees are overwritten with zeros, the database file is rebuilt, and its write-ahead log is
  emptied, so neither holds an erased value. Without the confirmation word, nothing is erased.

## What an erase does not reach

- **The backup replica** keeps erased data for at most 3 days (`P3D`): Litestream takes a
  snapshot every 24 hours and keeps each for 48 hours, so an erased row leaves the replica within
  72 hours.
- **The service journal** keeps its log lines for at most 14 days (`P14D`), and journald then
  deletes them.
- **The private copy of your collection** is your own Anki data: an erase leaves it, and the next
  sync would restore it from your sync server. Removing it means removing the sync credential, a
  step of the owner's setup.
- **The cron-fire ledger** records which scheduled jobs ran, and none of your data. It is kept
  after an erase and pruned after 90 days, so an erase can never make a notification send twice.

DeckStreak never uses your data to train a model.
