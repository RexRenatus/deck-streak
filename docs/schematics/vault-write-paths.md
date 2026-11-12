# Schematic: the vault adapter's two write paths

Kind: data flow. Read at DeckStreak `main` ce3683d (ADR-011, ADR-019, docs/schematics/data-flow.md),
at the predecessor's `27ee2bc` (`reading_notes.py`: `_atomic_write`, `_roll_note_text`,
`_move_with_roll`, `_archive_dir`, `roll_forward`, `stamp_studied`), and at the vault-duties pack as
vendored (its probe's `no-executable`, `write-confinement` and `never-deletes` classes, `rails.json`,
the run record and the three-verb grammar). Added by SPEC-042, and drawn again at its delivery with
the start check, the staged run's own checks before its gate, and the archive's numbered names
(ADR-042).

```mermaid
flowchart TD
  start["start check: root a directory; readings folder present, inside the root, writable; creates nothing"]
  start -- "refused" --> down["vault features refuse to start, by name"]
  start -- "ok" --> ready["the configured root, readings and archive folders, resolved"]

  subgraph staged["Agent duties: staged runs"]
    run["staging directory and duty-run.json"] --> own{"executor's own checks: three verbs only; plain paths; inside the duty's folders after links; no overwrite, case-insensitive; update and move hashes; staged set equals ops; rails"}
    own -- "refused" --> discard["run discarded, vault untouched"]
    own -- "ok" --> classes{"gate: every blocking vault-duties class, the pack's own probe"}
    classes -- "a class red, or no verdict" --> discard
    classes -- "green, or examined nothing of this duty" --> exec["apply create, update, move in order; hashes checked again at apply"]
  end

  subgraph tree["Readings date tree: written by DeckStreak's code"]
    create["create: predecessor's eight keys, ai_generated, body, two boxes"]
    rollf["roll-forward: most recent prior day's carried topics roll; every other prior-day note archives"]
    roll["roll: patch rolls, last_rolled, first_generated; write; read back; then remove source"]
    archive["archive: rename to Archive/YYYY/MM/Www/YYYY-MM-DD; a taken name gets -2, -3"]
    stamp["Studied stamp: exact line, settle step only"]
    tick["I read it tick: exact line, owner's tap only"]
    replace["body replacement: keep frontmatter and boxes; refuse an edited body"]
    rollf --> roll
    rollf --> archive
    rollf -- "malformed note" --> report["reported by path and bounded reason; siblings still move"]
  end

  ready --> tree
  exec --> rails{"rails: vendored rails.json, plus control characters"}
  create --> rails
  roll --> rails
  stamp --> rails
  tick --> rails
  replace --> rails
  rails -- "a rail matches" --> refused["no write; the rail and line are named, the text never echoed"]
  rails -- "clean" --> confine{"inside the configured folder after .. and links, not a link itself, not over a note?"}
  archive --> confine
  confine -- "no" --> refused
  confine -- "yes" --> atomic["temp .name.pid.tmp beside the target, fsync, rename, directory fsync"]
  atomic --> replica[("vault replica, synced to the owner's devices")]
```

| guard | where it holds | proved by |
|---|---|---|
| never a top-level folder | the start check; the executor refuses a missing top-level folder | SPEC-042 A11 |
| a temp name the bridge ignores | every write | SPEC-042 A1 |
| the rails | every byte written | SPEC-042 A2 and A3 |
| no overwrite, no delete | create, roll, archive, executor | SPEC-042 A8, A10 |
| nothing outside its folder | every write, after `..` and symbolic links | SPEC-042 A8 |
| the roll's bytes | the roll, proved against the predecessor | SPEC-042 A4, A5 |
| a roll-forward is idempotent | the roll-forward | SPEC-042 A7 |
| the owner's tick | only the tap writes it; nothing unticks | SPEC-042 A12, SPEC-047 A2 |
| the owner's edits | the body replacement | SPEC-042 A13 |
| one writer of the readings folder | the archive switch stays off until the predecessor's job is disabled | SPEC-053 A9 |
