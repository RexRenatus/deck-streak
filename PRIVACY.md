# DeckStreak privacy policy

DeckStreak serves one owner: the person who runs it, whose Anki reviews it scores. It keeps only
what it needs to run that study, in its own database. This page says what that is, why, on which
lawful basis, for how long, and what an erase leaves behind. [privacy.json](privacy.json) is the same
inventory, in the form the repository's privacy checks read.

## What DeckStreak keeps

| category | what | purpose | lawful basis | retention |
|---|---|---|---|---|
| `sync-history` | the record of each sync of your collection: when it ran, what started it, its outcome and its attempts | running your daily sync, and telling you when it stops | contract | until account deletion |
| `skip-days` | the record of each skip day: its study day, how many reviews were due, whether it was applied, failed or undone, and the cards it moved with their scheduling before and after | showing the skip, bridging your streak over it, and undoing it exactly | contract | until account deletion |
| `ingest-state` | what the last recompute of your reviews saw, whether you asked for a rescore, why and when a request of yours was refused, and the base of the review window | recomputing your scores only when your collection or your settings changed | contract | until account deletion |
| `service-counters` | one counter of how many times your settings changed, and a digest of your courses file | telling the recompute that a setting or your courses changed | contract | until account deletion |
| `xp-ledger` | every XP grant: the study day it pays for, what it was for, the track, the amount, and whether it pays once a day or once ever | keeping your XP total and level, and paying each award at most once | contract | until account deletion |
| `settled-xp` | the XP each study day earned, settled once per source and track, and whether the day had closed | keeping your XP total and level from your reviews and the day's bonuses | contract | until account deletion |
| `badges-earned` | each badge you earned, its day and when its celebration was sent | showing the badges you hold and sending each celebration once | contract | until account deletion |
| `personal-records` | your best daily score, most reviews and most minutes, the day set, the value beaten and when its celebration was sent | showing your personal records and sending each celebration once | contract | until account deletion |
| `day-buffs` | the buffs a study day holds and when each was armed | carrying the Ascendant bonus from the day that earned it to the day that gets it | contract | until account deletion |
| `reading-runs` | the record of each resolution of your readings for a study day: what started it, when it ran, its outcome and why, and how many decks with new cards matched no topic | resolving your daily readings, and showing you why a day has none | contract | until account deletion |
| `reading-topic-days` | each reading topic's state for each study day and why, and for a topic with new cards the ids of those cards and their notes | giving each topic one honest state each day, and the new cards its reading primes | contract | until account deletion |
| `agent-runs` | the record of each AI duty run: the duty, the persona template, the subject, how the run ended and why, the turns, tokens and cost it reported, and how long it took; never the prompt or the reply | showing you why a duty delivered nothing, and keeping each run's cost visible | contract | 90 days |
| `daily-rollups` | each study day's numbers from your reviews (answers by kind, time, true retention, graduations, decks studied), the card state recorded for it, its score and the score it closed with | scoring each study day once it closes, and showing you each day's numbers and score | contract | until account deletion |
| `daily-course-stats` | each course's reviews, time, first answers and passes on each study day | showing you how each day's work was split across your courses | contract | until account deletion |
| `course-progress` | each course's progress at the last recompute (its mastery, current band and unit, mature and total cards and each band's counts), and each band a course reached, its day, whether it was the course's first sighting and when its celebration was sent | showing your progress toward C2 in each course, and paying and celebrating each band you reach once | contract | until account deletion |
| `law-dues` | the law cards overdue and due on the study day at the last recompute, and that day | showing your law dues in the law block, and showing them as pending before the first count | contract | until account deletion |
| `notification-decisions` | every decision about a celebration or a nudge: its key, its kind, where it would go, whether it was sent, held or withheld and why, and the tiers asked for and shown | keeping one record of why each message was sent, held or withheld | contract | until account deletion |
| `notification-deliveries` | each message delivered: its key, its kind, where it went, the study day and the lapse it belongs to | sending each message at most once across the bot and the Mini App, and capping comebacks | contract | until account deletion |
| `notification-queue` | the celebrations held by quiet hours or a failed send: their text, why and since when they are held, and how often a send failed | delivering them once quiet hours end or sends succeed, and naming every one given up | contract | until account deletion |
| `in-app-feed` | the celebrations delivered to the Mini App, and when the Mini App fetched them | showing you in the Mini App the celebrations raised there | contract | until account deletion |
| `notification-settings` | which kinds of message you switched off, and your quiet hours | honouring your choices of what to receive and when to stay quiet | contract | until account deletion |
| `owner-last-message` | the id of your latest message to the bot and when it arrived | reacting to your latest message when a small celebration lands | contract | until account deletion |
| `streak-state` | each track's streak: the days running now, the longest run, the freezes held, the last study day and whether a comeback is armed | showing you your language and law streaks and keeping them from one study day to the next | contract | until account deletion |
| `freeze-events` | each freeze gained or spent: the study day, the change and why it happened | keeping the count of freezes you hold and limiting how many you can gain in a month | contract | until account deletion |
| `habit-strength` | the habit strength of each study day: the study day and its value | deciding whether the governor is armed and showing you how steady your habit is | contract | until account deletion |
| `relight-due` | the study days whose relight was granted and whose celebration is not yet decided | celebrating each relight once, also when DeckStreak restarts before the celebration is sent | contract | until account deletion |
| `governor-state` | the governor's one row: the day a lapse began, whether it is on standby and the day you were last told | keeping one lapse and one notice from repeating on every study day | contract | until account deletion |
| `coins` | every coin movement of the wallet: the study day it belongs to, what moved the coins and which one, the signed amount and when it was written | keeping your coin balance, its daily loss cap and its floor, and paying each movement at most once | contract | until account deletion |
| `economy-state` | the shop's one row: when the active scroll pass ends and when the pass surcharge ends | knowing whether a scroll pass is active and what the next one costs | contract | until account deletion |
| `chests` | every chest you earned: the study day, whether a study session, the daily challenge or the weekly quest earned it, the session's start, its rarity, the XP it pays, its state, the Epic prize you chose and whether its arrival was announced | rolling each chest once, paying it once on the study day it was earned, and never rolling the same session again | contract | until account deletion |
| `chest-pity` | the pity row: how many chests came since your last Epic and since your last Legendary | guaranteeing an Epic and a Legendary after a fixed run of chests | contract | until account deletion |
| `xp-tokens` | every double-XP token: the chest it came from, when it was granted, when it was activated, when its window ends and whether it is spent | activating each token once, opening its two-hour window, and paying its bonus at most once per study day | contract | until account deletion |
| `chest-settings` | the chest settings' one row: the most chests a study day holds and the hour from which a new chest is vaulted | honouring your choices of how many chests a study day holds and when they wait in the vault | contract | until account deletion |
| `law-drills` | which law drills you answered, from which surface and on which day, and each graded drill's type, subject, accepted XP and grading day; never a drill's text or your answer, which stay in your own notes | answering each drill once, paying each graded drill once, and telling the agent which drill types and subjects you practised | contract | until account deletion |
| `inbox-captures` | each capture you sent to your vault inbox from the bot or the Mini App: its name, its retry key, its kind, where it came from, its attachment's file name, when it was captured, and whether and where it was filed; the captions, texts and attachments themselves stay in your own vault | saving each capture once, answering a resend with the first capture's name, and letting the inbox curator file it; no model reads a capture unless the curator's route is configured, and a journal capture never | contract | until account deletion |
| `research-instruments` | the latest report of each research instrument that reads your collection: its study day, its findings about your own cards and note types, and any read that failed | showing you what each instrument last found, and running each weekly one once in seven study days | contract | until account deletion |
| `minutes-log` | each reading entry you logged: the course, the study day, the minutes, the note you added if any (up to 200 characters), and when it was written | settling each course's reading XP of the day and its weekly bonus from your entries, and removing the newest entry when you undo it | contract | until account deletion |
| `writing-log` | each study day you confirmed writing in a writing course: the course, the study day, and when it was confirmed | earning the writing XP and counting the writing streaks | contract | until account deletion |
| `passkeys` | each passkey you registered for signing in on the web: its credential id, the random user handle your passkeys share, the stored credential with its public key, the signature counter, the backup state, and when it was added and last used | signing you in on the web with a passkey | contract | until account deletion |
| `sensitive-decks` | the ids of the decks you keep away from AI, and when each was marked; never a deck's name or its cards | keeping every card of a deck you marked, and of the decks under it, away from every AI feature; after an erase every deck is readable again until you keep one away | contract | until account deletion |

The lawful basis of each is the contract (GDPR Article 6(1)(b)): the service you run needs it. None
is kept for a fixed period. Each is kept until you erase it, which is how an account is deleted here.

Outside its database, DeckStreak holds a private copy of your collection, your own Anki data,
refreshed from your sync server, and beside it the one backup of that copy a skip day's write makes
before it changes a card, which an erase removes and an export leaves out; when the owner runs the
sync server on the same host, that server's store (each sync user's collection, media index and
media files) and its daily snapshots; and, in memory only, your Telegram user id in a signed-in
session, and the bot's place in the queue of your messages. The bot reads the messages and button
taps you send it to answer them, keeps none of their text, and answers nobody but you.

In the web app, your browser keeps three things on your device, outside DeckStreak's database: the
copy of your collection the web app studies, its media files, and the key your sync login returns,
sealed so that it opens only under a second key the service releases to your own signed-in session,
beside the sync user name it belongs to. The sync password you type is used for that one login and
kept nowhere. Signing out of the web app removes the sealed key first and then ends your session,
so the key is gone even when your device is offline; clearing this site's data in your browser
removes all three. None of it is the service's store: an export of your data does not include it,
and only your browser holds it.

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
- **The edge's log of the sync route** records, for each request to the sync route, the client's
  address, the method, the path without its key and the status, and none of your Anki data. It is
  kept in the service journal for at most 14 days (`P14D`), and journald then deletes it.
- **The ban list** holds an address with five refused sync logins within ten minutes for up to one
  day after its ban, and then releases it.
- **The private copy of your collection** is your own Anki data: an erase leaves it, and the next
  sync would restore it from your sync server. Removing it means removing the sync credential, a
  step of the owner's setup.
- **The sync server's snapshots** are copies of the sync server's store, your own Anki data, taken
  each day while that server is stopped for a few seconds. An erase leaves them, as it leaves the
  server's store. The host keeps the newest three archives and removes the oldest when a fourth is
  in place; each archive is also sealed to a key that is never on the host, and only the sealed
  copy goes to an offsite bucket that admits no public access and no listing. It is deleted there
  30 days (`P30D`) after it is written, by a rule the owner sets on the bucket and the cutover
  runbook checks. Removing them is a step of the owner's setup.
- **The cron-fire ledger** records which scheduled jobs ran, and none of your data. It is kept
  after an erase and pruned after 90 days, so an erase can never make a notification send twice.

## Models and your data

DeckStreak does not train, fine-tune or fit a model on your data, and does not build a dataset from
it. The scheduling parameters in your collection come from your Anki app: DeckStreak reads them, and
the memory state Anki stores for each card, to schedule the cards you study, and does not fit them
to your reviews. An AI duty runs only when the host's AI route is configured, which it is not by
default. Each run sends the cards that duty covers and a summary of your leeches, lapses and graded
practice, not your journal, as the context of that one run, and DeckStreak's database keeps neither
the prompt nor the reply. A reply that passes its checks is written to your own vault. What the
model's provider keeps is set by that provider's terms.
