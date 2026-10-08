The owner amends SPEC-357 R7's re-check stamp, choosing "(B) usn+schema stamp" in the owner's own words: an upload's re-check compares the server copy's ids and one integer, `fnvhash` of the greatest row usn over every synced table that carries one (graves included) and the collection's schema stamp `col.scm`, in place of the collection's modified stamp. It may be narrower than the modified stamp only for a client that writes a row usn below the server's, which the sync protocol never does. Its full-upload arm relies on the uploader's millisecond schema stamp, as the modified stamp it replaces already relies on the uploader's clock.

# OWNER RULING 2026-10-08: the upload re-check stamp

## What was held

SPEC-357 R7 (`docs/specs/SPEC-357-the-full-sync-choice-and-the-web-sync-screens.md:113-117`) lets an upload reach
`Ready` only from a fresh server copy whose ids and modified stamp equal the counted copy's. The modified stamp cannot
do that job:

| what the stamp must do | the modified stamp `col.mod` | why it matters |
|---|---|---|
| stay still when nothing changed | moves: every download restamps it at the server's clock | an unchanged server always reads as changed, so a re-check can never pass |
| move on every write another client makes through a normal sync | moves, but only on the server's own clock | holds |
| move on every full upload | moves at the uploader's clock | holds, through a client clock |

The two measured replacements each failed one test:

- **The greatest row usn alone.** It stays still across a download and moves on a normal sync. A full upload, though,
  resets the uploader's changed rows to usn 0 and leaves every unchanged row's usn alone. So an uploaded edit never
  moves the greatest while any unchanged row still holds it.
- **A digest of every row.** It meets every condition for every write that changes a row. SQL cannot compute it,
  because the engine's only SQL hash reads integers and the rows hold text. So the core would compute it in Rust,
  reading the whole collection five times per upload in the web worker. That cost was measured and rejected.

## What replaces it

- **The stamp.** One fixed integer statement: `fnvhash` of the greatest row usn over every synced table that carries
  one, graves included, and `col.scm`. The re-check compares the ids and this value. `rechecked` keeps its shape: it
  still compares a fresh copy with the counted one.
- **What moves it:**
  - A normal sync. The server stamps tags, config and graves with its next usn. An engine client stamps every other
    row it sends with the server's usn.
  - A full upload. The uploader sets `col.scm` to its millisecond clock.
- **What does not move it.** A download's restamp, which changes only `col.usn`, `col.mod` and `col.ls`.
- **The admitted reliance, and only this:**
  - The full-upload arm relies on two uploads not carrying the same millisecond schema stamp. The modified stamp
    already relies on the uploader's clock.
  - The stamp is narrower than the modified stamp for a client that writes a row usn below the server's. The sync
    protocol never writes one.
- **What the build must prove.** For each condition there is a test:
  - a row edit that adds no id moves the stamp;
  - a tag-only change and a config-only change each move it;
  - a deletion moves it;
  - an edit whose modified time is at or below the greatest row's moves it;
  - a full upload moves it;
  - an unchanged server's download leaves it still.

  For each synced table, a mutation row drops that table from the statement and is killed.

## Why it is admitted

It is one cheap integer statement. It keeps part a's tests, the formal model of the full sync choice, and the
statement shape the core already reads. The reliance it adds is the same kind the stamp it replaces already carries.

The rejected alternatives:

- **The digest.** It is clock-free, but it costs five full reads per upload in the web worker. The owner chose the
  stamp.
- **Leaving R7 as written.** It would block every upload re-check, because a download always moves the modified stamp.
- **The greatest row usn alone.** It misses an uploaded edit whenever an unchanged row holds the greatest usn.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
