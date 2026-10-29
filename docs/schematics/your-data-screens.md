# Your data and this device's backups: the data flow (SPEC-516, ADR-534)

Three flows leave the data screens. Each arrow is a call; each box names the file that makes it.

## The JSON copy (R2, R3)

```mermaid
flowchart LR
  screen["web data screen<br/>web/app/src/lib/data/DataScreen.svelte"]
  api["the app's API client<br/>web/app/src/lib/api.ts"]
  route["POST /api/data/export<br/>crates/api/src/data_export_routes.rs"]
  guard["state-change guard, owner session,<br/>window, body bound, database"]
  doc["export_document<br/>crates/coordination/src/data_rights_registry.rs"]
  ports["export_all over every port<br/>crates/privacy"]
  role["deckstreakd data export<br/>crates/daemon/src/role_data.rs"]
  file["deck-streak-export.json<br/>saved by the browser"]
  screen --> api --> route --> guard --> doc --> ports
  role --> doc
  route -- "the line and its newline" --> screen --> file
```

The route and the host's role call one function, so the bytes the browser saves are the bytes the
host writes (D2). The native screen has no owner session yet, so it names the web screen and
`/export` instead (D7).

## Back up now and the list, on the web (R4)

```mermaid
flowchart LR
  screen["web data screen"]
  worker["the Worker<br/>web/app/src/lib/engine/session.ts, backups.ts"]
  wasm["back_up_now, backups, retain, export_backup<br/>crates/web-engine/src/wasm.rs"]
  names["files.rs: a new Backup-kind name"]
  core["backup_now::back_up<br/>crates/engine-core/src/backup_now.rs"]
  pool["the browser's pool: backup-n.anki2"]
  keep["retention::removals<br/>crates/engine-core/src/retention.rs"]
  screen -- "backup-now" --> worker --> wasm --> names
  wasm --> core -- "VACUUM INTO, read back" --> pool
  screen -- "backups" --> worker -- "retain, then list" --> keep
```

## Back up now, the list and the export, on iPhone and iPad (R5)

```mermaid
flowchart LR
  view["DataView.swift, DataModel.swift"]
  session["EngineSession.swift"]
  adapter["Engine.back_up_now, device_backups, backup_bytes<br/>crates/ffi/src/backups.rs"]
  core["backup_now::back_up"]
  keep["retention::removals"]
  folder["the collection's folder:<br/>backup-n-p.anki2, server-copy-n-p.anki2"]
  exporter["fileExporter: the system's save dialog"]
  view --> session --> adapter
  adapter --> core --> folder
  adapter -- "list, then keep three of each kind" --> keep
  adapter -- "the listed id's bytes" --> view --> exporter
```

## The server's backup age (R6)

The web screen reads `GET /api/sync/snapshot` (SPEC-377 R12): an age when found, "no backup yet"
when not found, and "unknown" for `{"found": null}`, a refusal or a failed read. The native screen
shows "unknown" while it holds no owner session.
