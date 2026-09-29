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
| `service-counters` | one counter of how many times your settings changed, and a digest of your courses file | telling the recompute that a setting or your courses changed | contract | until account deletion |
| `xp-ledger` | every XP grant: the study day it pays for, what it was for, the track, the amount, and whether it pays once a day or once ever | keeping your XP total and level, and paying each award at most once | contract | until account deletion |
| `reading-runs` | the record of each resolution of your readings for a study day: what started it, when it ran, its outcome and why, and how many decks with new cards matched no topic | resolving your daily readings, and showing you why a day has none | contract | until account deletion |
| `reading-topic-days` | each reading topic's state for each study day and why, and for a topic with new cards the ids of those cards and their notes | giving each topic one honest state each day, and the new cards its reading primes | contract | until account deletion |
| `agent-runs` | the record of each AI duty run: the duty, the persona template, the subject, how the run ended and why, the turns, tokens and cost it reported, and how long it took; never the prompt or the reply | showing you why a duty delivered nothing, and keeping each run's cost visible | contract | 90 days |
| `daily-rollups` | each study day's numbers from your reviews (answers by kind, time, true retention, graduations, decks studied), the card state recorded for it, its score and the score it closed with | scoring each study day once it closes, and showing you each day's numbers and score | contract | until account deletion |
| `daily-course-stats` | each course's reviews, time, first answers and passes on each study day | showing you how each day's work was split across your courses | contract | until account deletion |
| `notification-decisions` | every decision about a celebration or a nudge: its key, its kind, where it would go, whether it was sent, held or withheld and why, and the tiers asked for and shown | keeping one record of why each message was sent, held or withheld | contract | until account deletion |
| `notification-deliveries` | each message delivered: its key, its kind, where it went, the study day and the lapse it belongs to | sending each message at most once across the bot and the Mini App, and capping comebacks | contract | until account deletion |
| `notification-queue` | the celebrations held by quiet hours or a failed send: their text, why and since when they are held, and how often a send failed | delivering them once quiet hours end or sends succeed, and naming every one given up | contract | until account deletion |
| `in-app-feed` | the celebrations delivered to the Mini App, and when the Mini App fetched them | showing you in the Mini App the celebrations raised there | contract | until account deletion |
| `notification-settings` | which kinds of message you switched off, and your quiet hours | honouring your choices of what to receive and when to stay quiet | contract | until account deletion |
| `research-instruments` | the latest report of each research instrument that reads your collection: its study day, its findings about your own cards and note types, and any read that failed | showing you what each instrument last found, and running each weekly one once in seven study days | contract | until account deletion |

The lawful basis of each is the contract (GDPR Article 6(1)(b)): the service you run needs it. None
is kept for a fixed period. Each is kept until you erase it, which is how an account is deleted here.

Outside its database, DeckStreak holds a private copy of your collection, your own Anki data,
refreshed from your sync server; and, in memory only, your Telegram user id in a signed-in session,
and the bot's place in the queue of your messages. The bot reads the messages and button taps you
send it to answer them, keeps none of their text, and answers nobody but you.

## Your copy of your data, and erasing it

From Telegram, the bot answers two commands, and only yours:

- `/export` sends you every category above as one JSON document, the same one the host writes
  below. The document goes to your chat through Telegram, which keeps it there until you delete it
  in Telegram.
- `/delete` asks you to confirm with a button, and erases exactly as the host's erase below does
  once you tap it. Only the button on the latest question erases, and only once.

You can also export and erase on the host that runs DeckStreak:

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
- **The daily copies of the database** keep erased data for at most 3 days (`P3D`): one copy is
  made each day, the newest three are kept, and the oldest is removed when a fourth is in place, so
  an erased row leaves them within 3 days. They hold the database and nothing else: not the
  private copy of your collection.
- **The service journal** keeps its log lines for at most 14 days (`P14D`), and journald then
  deletes them.
- **The private copy of your collection** is your own Anki data: an erase leaves it, and the next
  sync would restore it from your sync server. Removing it means removing the sync credential, a
  step of the owner's setup.
- **The cron-fire ledger** records which scheduled jobs ran, and none of your data. It is kept
  after an erase and pruned after 90 days, so an erase can never make a notification send twice.

DeckStreak never uses your data to train a model.
