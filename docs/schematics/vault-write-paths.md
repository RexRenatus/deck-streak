# Schematic: the vault adapter's two write paths

Kind: data flow. Read at DeckStreak `main` ce3683d (ADR-011, ADR-019, docs/schematics/data-flow.md),
at the predecessor's `27ee2bc` (`reading_notes.py`: `_atomic_write`, `_roll_note_text`,
`_move_with_roll`, `_archive_dir`, `roll_forward`, `stamp_studied`), and at the packs vendored from
`19bb0f3` (vault-duties: `rails.json`, the run record, the three-verb grammar). Added by SPEC-042.

```mermaid
flowchart TD
  subgraph staged["Agent duties: staged runs"]
    run["staging directory and duty-run.json"] --> classes{"vault-duties blocking classes"}
    classes -- "any red" --> discard["run discarded, vault untouched"]
    classes -- "all green" --> exec["executor: create, update, move only"]
  end
  subgraph tree["Readings date tree: written by DeckStreak's code"]
    create["create: predecessor's eight keys, ai_generated, body, two boxes"]
    roll["roll: patch rolls, last_rolled, first_generated; read back; then remove source"]
    archive["archive: rename to Archive/YYYY/MM/Www/YYYY-MM-DD"]
    stamp["Studied stamp: exact line, settle step only"]
    tick["I read it tick: exact line, owner's tap only"]
    replace["body replacement: keep frontmatter and boxes; refuse an edited body"]
  end
  exec --> rails{"rails from vendored rails.json"}
  create --> rails
  roll --> rails
  stamp --> rails
  tick --> rails
  replace --> rails
  rails -- "a rail matches" --> refused["no write; the rail is named, the text never echoed"]
  rails -- "clean" --> confine{"inside the configured folder, and not over a note?"}
  archive --> confine
  confine -- "no" --> refused
  confine -- "yes" --> atomic["temp .name.pid.tmp, fsync, rename, directory fsync"]
  atomic --> replica[("vault replica, synced to the owner's devices")]
```

| guard | where it holds | proved by |
|---|---|---|
| never a top-level folder | the start check | SPEC-042 A11 |
| a temp name the bridge ignores | every write | SPEC-042 A1 |
| the rails | every byte written | SPEC-042 A2 and A3 |
| no overwrite, no delete | create, archive, executor | SPEC-042 A8, A10 |
| the owner's tick | only the tap writes it; nothing unticks | SPEC-042 A12, SPEC-047 A2 |
| one writer of the readings folder | the archive switch stays off until the predecessor's job is disabled | SPEC-053 A9 |
