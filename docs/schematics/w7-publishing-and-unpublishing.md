# Schematic: publishing the achievement page, and withdrawing it

Kind: state machine, data flow and sequence. Read at DeckStreak `dev` 026d1f3 (ADR-012, ADR-027,
ADR-059, `docs/schematics/cron-fire-ledger-and-catch-up.md`,
`docs/schematics/data-rights-export-and-erase.md`, `docs/schematics/deployment.md`,
`docs/schematics/first-deploy-and-gates.md`). Added by the W7 architect turn for SPEC-137
(ADR-137) and SPEC-130's settings screen (ADR-130). It rewrites none of them: the job is one row of
the cron-fire ledger, and the erase keeps the engine drawn in `data-rights-export-and-erase.md`,
with the withdrawal as its first step. DeckStreak serves the page itself from a directory the owner
configures, so a withdrawal is a file removal on its own host and nothing public outlives it.

## The switch

```mermaid
stateDiagram-v2
  [*] --> off: the default, and after the import
  off --> off: switching on while the directory is unset, refused publishing_unconfigured
  off --> on: the owner confirms on the settings screen
  on --> off: switching off withdraws every file and deletes the records, in one transaction
  on --> off: either erase path resets the switch in place
```

The screen names, before the owner confirms, what becomes public (the wired sections of the typed
page) and what never does (the named drops). No bot command publishes or unpublishes.

## One run of the job

```mermaid
flowchart TD
  fire["job public_page, daily at rollover minute 36, catch-up on"] --> sw{"switch on"}
  sw -->|no| z["writes nothing, exits 0"]
  sw -->|yes| dir{"public directory configured"}
  dir -->|no| pg["exits with the page code"]
  dir -->|yes| each["each page: the language page at the root, the law page under law"]
  each --> comp["compose the typed page: only its allow-listed fields"]
  comp --> wired{"any wired section"}
  wired -->|no| nw["not_written"]
  wired -->|yes| chk{"catalogs, study days, the scrubber"}
  chk -->|a value outside its catalog| r1["not_in_catalog"]
  chk -->|a study day after the page's day| r2["future_date"]
  chk -->|the scrubber finds a marker or a secret shape| r3["scrub_refused"]
  chk -->|clean| hash{"content hash equals the recorded one"}
  hash -->|yes| un["unchanged, nothing written"]
  hash -->|no| write["one BEGIN IMMEDIATE: temp name, rename, stale files removed, each file recorded"]
  write -->|a failed write| rb["the records roll back, the next run writes again"]
  write -->|done| pub["published"]
  r1 --> keep["that page's last clean files and records stay, the job exits with the page code"]
  r2 --> keep
  r3 --> keep
  rb --> keep
```

A refusal of one page never stops the other page. A second run on the same study day writes
nothing, because the hash is unchanged. `POST /api/publishing/run` runs the same composition and
write at once for the owner, and answers each page's outcome by name.

## What is written, and where it is served

```mermaid
flowchart LR
  subgraph dir["the configured public directory"]
    lang["stats.json, badges, page.css, index.html"]
    law["law: stats.json, badges, page.css, index.html"]
  end
  rec[("published_files: page, path, SHA-256, length, content, instant")] --- dir
  caddy["the site block: handle_path /public, file_server, no fallback"] --> dir
  caddy --> hdr["no-cache, and a policy of its own: no script, styles from self, no framing"]
```

Files are written `0644` in `0755` directories whatever the unit's umask. A page holds no script,
no inline style, no form and no resource from another origin. A withdrawn file answers 404, since
the handle has no fallback. A page recorded under a directory the configuration no longer names is
withdrawn from that directory before it is written under the new one.

## The unpublish: switching off, and the erase

```mermaid
sequenceDiagram
  participant O as the owner
  participant C as coordination
  participant P as publishing
  participant D as the public directory
  participant E as the erase engine
  O->>C: the bot's delete, or the data role's erase
  C->>P: withdraw_published
  P->>D: remove each recorded file, a missing file counts as withdrawn
  alt a file cannot be removed
    P-->>C: withdraw_failed, nothing erased, the records still name what is public
  else every file is gone
    P-->>C: withdrawn
    C->>C: SPEC-131's revocation step, once it has landed
    C->>E: erase_all: published_files emptied, the switch reset to off
  end
```

Export holds each published file's bytes as written, with its path and instant, and the switch;
erase empties what export lists, so export and erase stay symmetric (SPEC-021).
