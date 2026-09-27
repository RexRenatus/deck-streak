---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/privacy-gdpr

What a repository must prove about the personal data it keeps, before it is published and before
it ships (SPEC-V2-2222 / ADR-V2-2222). The repository declares its data once, in `privacy.json`,
and fifteen checks prove the declaration against the tree itself: the SQL migrations, the export,
erase and purge code, the Litestream and journald configuration, the privacy policy, the default
settings, and every file the repository publishes.

It covers the GDPR's principles and records (Arts 5, 6 and 30), the learner's rights (Arts 13, 15,
17 and 20), privacy by design and by default (Art. 25), Telegram's own rules for the data a bot or
Mini App receives, and the rule for a PUBLIC repository: no address, cloud id, secret name, chat id
or personal data in any published file.

`scripts/privacy-gdpr-probe.py` is the check. It is standard-library Python and vendorable, it
judges any tree through `--root`, and every row below runs it. The public-repository scrubber
composes with persona-core's scrubber: it loads persona-core's probe and calls its `load_deny()`
for persona-core's deny list (public and private), and for this pack's own public-repository
shapes, which live in `deny-list.json` in persona-core's deny schema. Nothing of persona-core is
copied. Which seats consume this pack is its catalog row's `consumes`, so this body names none.

```
phxd pack probe --pack privacy-gdpr --root PATH --format json
```

## The rows

Fifteen rows, all `tree`-scoped, one per class of `privacy-gdpr-probe.py`. Each runs
`python3 {skills}/../scripts/privacy-gdpr-probe.py --root {root} check <class>` under a 120-second
wall, and the two `public` rows under 300 seconds. The script finds every `privacy.json` under the
root (dot-directories, `node_modules` and `target` skipped) and judges each against the tree
under its own directory, so a repository keeps one at its root.

The `inventory` stage: 4 rows. They read `privacy.json` and the SQL schema it names.

| row | severity | reason | refuses when |
|---|---|---|---|
| `inventory-valid` | block | `inventory-invalid` | `privacy.json` is not JSON or not `phx.privacy.v1`; a key is unknown or missing; a category has no id, data, source, stores, purpose, lawful basis, retention, export or erase; an id repeats; a source or an Art. 6(1) basis is outside the vocabulary; a store is not `table.column`, `table.*` or `cloudstorage:key`; a named file or glob matches nothing |
| `schema-covered` | block | `personal-data-undeclared` | a column of a personal table is in no category, where a table is personal when its name is a person's (`users`, `accounts`), when it holds a user link (`user_id`, `telegram_id`, `chat_id`) or a direct identifier (`first_name`, `username`, `email`, `phone`), or when it indexes such a table; a store names a table or column no migration creates; the Mini App writes a `CloudStorage`, `DeviceStorage` or `SecureStorage` key no category declares, or a dynamic key without `cloudstorage:*` |
| `retention-bounded` | block | `retention-unbounded` | a category's retention is neither an ISO 8601 period (with an optional `after` event) nor `until` a known event: account deletion, consent withdrawal, purpose end, last activity or collection (Art. 5(1)(e)) |
| `purpose-limited` | block | `purpose-unlimited` | a consent category names no `withdraw` path (Art. 7(3)); a legitimate-interests category has no `balancing` note (Art. 6(1)(f)); a category of Telegram data names training, fine-tuning, datasets or machine learning as its purpose (Telegram Bot Developer Terms s4.3) |

The `rights` stage: 4 rows. They read the export, erase and policy the inventory names.

| row | severity | reason | refuses when |
|---|---|---|---|
| `export-complete` | block | `export-incomplete` | the export format is not JSON, CSV or XML (Art. 20(1)); a table of an exportable category is never read (`FROM` or `JOIN`) by the export code, comments ignored (Art. 15(3)) |
| `erase-complete` | block | `erase-incomplete` | a table of a `delete` category is never in a `DELETE FROM`, or of an `anonymise` category in no `DELETE FROM` or `UPDATE`; a client-storage key is never removed with `removeItem`; a `retain` category cites no Art. 17(3) exception |
| `erase-effective` | block | `erase-recoverable` | no code turns SQLite's `secure_delete` on and the erase runs no `VACUUM`, so erased rows stay in free pages (`FAST` does not count: it leaves freelist traces); a personal full-text table gets neither its own `'secure-delete'` option nor an `'optimize'` or `'rebuild'` in the erase |
| `policy-published` | block | `policy-incomplete` | no line of the policy names a category with its lawful basis and its retention (as the ISO period or in words); the policy never states the backup or log windows; an entry point's file does not carry its text, so the policy is not reachable there (Art. 13, Telegram Bot Developer Terms s4) |

The `design` stage: 5 rows (4 blocking, 1 advisory). They read the timer, the copies, the
Telegram fields, the defaults and the logging code.

| row | severity | reason | refuses when |
|---|---|---|---|
| `purge-automated` | block | `purge-not-automated` | a category keeps data for a fixed period and no purge is declared, the purge timer has no `OnCalendar=` or monotonic schedule, or the purge code never runs `DELETE FROM` on one of that category's tables (EDPB 4/2019 para 82: automate deletion) |
| `copies-bounded` | block | `copy-retention-unbounded` | Litestream's snapshot interval plus its global `snapshot.retention` (each 24h by default) outlast the declared backups window; a replica-level `retention` is set, which Litestream 0.5 ignores; journald sets no `MaxRetentionSec`, or one past the declared logs window |
| `telegram-minimised` | block | `telegram-overcollected` | a column stores raw launch data (`init_data` and its spellings); an optional Telegram field (`last_name`, `username`, `photo_url`, `is_premium`, `phone_number`, `allows_write_to_pm`) is stored with no `justify` reason; the Mini App calls `requestContact` and no category declares a phone store |
| `defaults-private` | block | `default-exposes-data` | no default settings file is named in `config`; a setting under a sharing, public, visibility, leaderboard, analytics, telemetry or tracking key is on by default and not listed in `public_by_design` with a reason (Art. 25(2)) |
| `logs-pseudonymised` | advisory | `personal-data-logged` | a log call (`tracing` macros, `console`, `logging`) records a name, username, contact, photo, launch data, address or user agent outside a string literal. Advisory: it reports and never refuses |

The `public` stage: 2 rows. They read every file the repository publishes: git's tracked and
unignored files in a checkout, or every file otherwise.

| row | severity | reason | refuses when |
|---|---|---|---|
| `public-scrub` | block | `public-scrub-denied` | a line matches persona-core's deny list (its public value shapes: IPv4, tokens, keys, emails, Telegram bot tokens and supergroup ids; its private patterns and literals) or this pack's shapes: IPv6, a cloud project id on a command line, in a key or in a resource path, a secret name, a bucket, a cloud service host, an internal host, a home directory that names a user, a Telegram chat or user id or a phone number written in context |
| `private-list-committed` | block | `private-list-exposed` | a published JSON file is a deny list (`phx.persona.deny.v1`) with private literals, or a published file sits under a path the inventory declares `private` |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
population it read. A finding names the rule and the file and line, never a denied value. The
script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when VOID: no inventory was
found, or an input could not be read, which is never a pass. The card turns a `block` row's
non-zero exit red, and prints an `advisory` row's failure as `advisory` without reddening the card.

## The inventory, `privacy.json`

One file, at the repository root. Its top-level keys:

| key | holds |
|---|---|
| `schema` | `phx.privacy.v1` |
| `categories` | the categories of personal data, below |
| `sql` | globs of the SQL migrations, read in path order: `CREATE TABLE`, `CREATE VIRTUAL TABLE`, `ALTER TABLE` (add, rename, drop) and `DROP TABLE`; comments ignored |
| `code` | globs of the source the checks read for storage calls, `secure_delete`, `requestContact` and log calls |
| `export`, `erase`, `purge` | `{code: [globs]}`; `export` adds `format`, `purge` adds `timer` |
| `backups`, `logs` | `{config: file, retention: ISO period}`: the Litestream file and the journald drop-in |
| `policy` | `{file, entry: [{file, text}]}`: the policy, and where the product links to it |
| `config` | globs of the default settings files, TOML or JSON |
| `public_by_design` | `{setting: reason}`, a setting that is on by default on purpose |
| `public` | `{allow: [literal]}`, a value the scrubber may pass, such as a public security contact |
| `private` | globs that must never be published: a private roster, a private deny list |

A category holds `id`, `data` (what it is, in words), `source`, `stores`, `purpose`,
`lawful_basis`, `retention`, `export`, `erase`, and where it applies `withdraw`, `balancing` and
`justify`.

- **Sources:** `telegram-initdata`, `telegram-update`, `telegram-storage`, `learner-input`,
  `study-activity`, `derived`, `vault`, `operator`. The three Telegram sources carry Telegram's
  rules.
- **Stores:** `table.column`, `table.*`, `cloudstorage:key`, `devicestorage:key`,
  `securestorage:key`, or `cloudstorage:*` for keys the code builds at run time.
- **Retention:** `{"period": "P2Y", "after": "collection"}` or `{"until": "account-deletion"}`.
- **Export:** `true`, or `{"exempt": "why"}` for data another category already exports.
- **Erase:** `delete`, `anonymise`, or `{"retain": "<Art. 17(3) exception>", "why": "..."}`.

The vocabulary behind these lists is `vocabulary.json`: a new column name, source or exposure key
is a row there, never a code branch.

## What the checks prove, and how they read

- **Personal tables.** A table is personal by its name, a user-link column, a direct identifier, or
  a full-text `content=` table that is personal. Every one of its columns must be in a category, so
  study history tied to a learner is declared even though no column is a name.
- **Code references are statements, not words.** The export must read a table with `FROM` or `JOIN`,
  the erase must `DELETE FROM` it (or `UPDATE` it when anonymising), and the purge must
  `DELETE FROM` it. Comments are stripped first, so a table named in a comment is not exported.
- **Erasure that holds.** SQLite overwrites deleted content only with `PRAGMA secure_delete = ON` (or
  `.pragma("secure_delete", "on")` on sqlx's connect options), or after a `VACUUM`. An FTS5 index
  keeps deleted rows readable until a merge unless its own `'secure-delete'` option is set, which
  needs SQLite 3.42.0 or later and the core pragma beside it.
- **Copies.** Litestream 0.5 keeps snapshots for its global `snapshot.retention`, and ignores a
  `retention` under a replica. An erased row can stay in a replica for the interval plus the
  retention, so the declared backups window must cover both, and the policy must say it. journald
  keeps logs with no time limit unless `MaxRetentionSec=` is set.
- **The policy.** One line per category that names its id, its lawful basis and its retention, the
  way a table row does. The retention may be written as the ISO period (`P2Y`) or in words
  (`2 years`, `until account deletion`).
- **The public scrub.** Loopback, unspecified, link-local, multicast, broadcast and the RFC 5737
  and RFC 9637 documentation addresses pass. A dotted number that continues (an OID or a version)
  is not an address. A placeholder (`$VAR`, `${VAR}`, `<var>`, `%VAR%`) or an example project id
  never matches. Name the owner's own ids and words in persona-core's PRIVATE deny list, through
  `--deny-list FILE` or `PERSONA_CORE_DENY_LIST`, on the machines that hold it; CI on the public
  repository judges by the public shapes.

## Telegram user data

What a bot or Mini App receives, and what Telegram's Bot Platform Developer Terms require of it:

- **What arrives.** A Mini App's validated `initData` carries a `user` (id, first name, and
  optionally last name, username, language code, premium flag, photo URL and whether the bot may
  write to them), plus `auth_date`, `hash` and `signature`. A bot's updates carry the same user
  object. Validate `initData` on the server, keep only the fields a category needs, and never store
  the raw string.
- **Essential and optional.** The user id is the account; the first name and language code address
  the learner. Every other field is optional and needs a `justify` reason: that is data
  minimisation, Art. 5(1)(c), and the Terms' s4.3, which forbids collecting data beyond what the
  service needs.
- **Client storage.** `CloudStorage` lives on Telegram's servers, `DeviceStorage` on the device, and
  `SecureStorage` in the device keystore. A key the Mini App writes is personal data like a column,
  so it is declared, exported where it can be, and removed on erasure.
- **A phone number** arrives only through `requestContact`, when the learner shares it. Declare it
  and justify it, or never ask.
- **No datasets.** The Terms forbid any data collection aimed at building datasets, machine learning
  models or AI products. The learner's data may reach a model only as context for their own
  request, never as training data.
- **Deletion.** The Terms require deletion on the user's request, when the data is no longer
  needed, and when the service stops; `erase-complete` and `purge-automated` check the first two.
- **Privacy policy.** Every bot and Mini App is bound by a privacy policy the user can easily reach.
  Telegram's Standard Privacy Policy applies when the developer publishes none, so DeckStreak
  publishes its own and links it from `/privacy`.

## What no static read can prove, taught here

The checks read the tree. These bind the running service, and the engine's own tests or the
operator carry them:

- **Answer within one month** of a request, extendable by two when complex (Art. 12(3)), free of
  charge unless manifestly unfounded or excessive (Art. 12(5)).
- **Verify identity** before an export or an erasure, through the Telegram user id the validated
  `initData` carries.
- **Prove the export is whole.** An engine test exports a seeded learner and asserts every category
  appears in the JSON. The static check proves the tables are read, not that each result is
  written.
- **Prove erasure happened.** An engine test erases a seeded learner and asserts no row, index entry
  or client-storage key remains; keep the test's output as the evidence.
- **Backups past their window** are put beyond use and not restored for any other purpose, and the
  learner is told what happens to them.
- **Direct transmission** to another controller (Art. 20(2)) where technically feasible.
- **Encryption at rest, stored apart from its key** (Terms s4.4(a), Art. 32), and breach alerts to
  users (Terms s4.4(b), Arts 33 and 34): the host's disk encryption and the operator's incident
  procedure.
- **A DPIA** (Art. 35) when the processing is likely to be high-risk: a written assessment, not a
  tree property.

## How DeckStreak applies it

1. **Declare.** Copy `templates/privacy.template.json` to `privacy.json` at the root. Add one
   category per kind of data: the account, study history, notes, the vault features, client
   storage. Name the migrations, the export, erase and purge code, the Litestream file, the
   journald drop-in, the policy and the default settings.
2. **Build what it names.** Start the erase from `templates/erase.template.sql`: `secure_delete` on,
   the FTS5 `'secure-delete'` option, and a `DELETE FROM` per table. Add a daily purge timer. Set
   Litestream's global snapshot retention from `templates/litestream.template.yml` and journald's
   `MaxRetentionSec` from `templates/journald-retention.template.conf`. Write the policy from
   `templates/PRIVACY.template.md`, one table row per category, and register `/privacy`, `/export`
   and `/delete` with the bot.
3. **Run it.** From this repository against DeckStreak's tree:

   ```
   phxd pack probe --pack privacy-gdpr --root PATH --format json
   ```

   Or vendor `scripts/privacy-gdpr-probe.py` with this pack's `vocabulary.json` and `deny-list.json`,
   and persona-core's probe and deny list beside them, and run each class in CI:
   `python3 scripts/privacy-gdpr-probe.py --root . check <class>`.
4. **Gate the push.** On the owner's machine, run `public-scrub` with the private deny list named, so
   the owner's own ids and words are caught before they reach the public repository.

What the gate refuses: an undeclared personal column or storage key; an unbounded retention; consent
with no way out; Telegram data kept for training; an export or erase that skips a table; erased rows
left in free pages or an index; a policy that omits a category or the backup window; no purge
timer; snapshots or logs that outlive their window; raw `initData`; an unjustified optional
Telegram field; data public by default; and any address, cloud id, secret name, chat id, personal
email or private list in a published file.

`examples/deckstreak/` is the worked example, green on every row.

## References

The dated research, with access dates and the Context7 ids that answered, is SPEC-V2-2222's
References section.

- GDPR, Regulation (EU) 2016/679: https://eur-lex.europa.eu/eli/reg/2016/679/oj ·
  Arts 5, 6, 12, 13, 15, 17, 20, 25 and 30 at https://gdpr-info.eu/
- EDPB Guidelines 4/2019 on Article 25:
  https://www.edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-42019-article-25-data-protection-design-and_en
- EDPB Guidelines 01/2022 on the right of access:
  https://www.edpb.europa.eu/our-work-tools/our-documents/guidelines/guidelines-012022-data-subject-rights-right-access_en
- EDPB coordinated enforcement on the right to erasure:
  https://www.edpb.europa.eu/our-work-tools/our-documents/other/coordinated-enforcement-action-implementation-right-erasure_en
- ICO: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/individual-rights/right-to-erasure/ ·
  https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/individual-rights/right-to-data-portability/ ·
  https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-protection-principles/a-guide-to-the-data-protection-principles/storage-limitation/
- Telegram: https://telegram.org/tos/bot-developers · https://telegram.org/privacy-tpa ·
  https://core.telegram.org/bots/webapps
- SQLite: https://www.sqlite.org/pragma.html#pragma_secure_delete · https://www.sqlite.org/fts5.html
- Litestream: https://litestream.io/reference/config · https://litestream.io/docs/migration
- systemd: https://www.freedesktop.org/software/systemd/man/latest/journald.conf.html
- OWASP Logging Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- Google Cloud identifiers: https://docs.cloud.google.com/resource-manager/docs/creating-managing-projects ·
  https://docs.cloud.google.com/secret-manager/docs/creating-and-accessing-secrets
- Documentation addresses: https://www.rfc-editor.org/rfc/rfc5737.html · https://www.rfc-editor.org/rfc/rfc9637.html
