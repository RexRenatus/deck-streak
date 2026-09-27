# Importing the v9 database

The port runs once, from a read-only copy of the source, into a new target database. Every command
below runs in this order; `data-migration`'s rows read this file's code blocks in order.

## 1. Pin the source

Record the source's version and its whole live schema in one snapshot, then commit it. The plan's
`source.user_version` must equal the pin on its first line.

```sh
{ printf 'PRAGMA user_version = %s;\n' "$(sqlite3 -readonly /var/lib/v9/v9.db 'PRAGMA user_version')"
  sqlite3 -readonly /var/lib/v9/v9.db .schema; } > migration/source-schema.sql
```

The snapshot is the live schema, including every table a stepped migration added, never the
bootstrap DDL in the source's code.

## 2. Dry run

Without `--apply` the tool reads the source, plans every table and prints the counts it would write,
and writes nothing.

```sh
cargo run --release -p v9-import -- --source /var/lib/v9/v9.db --target /var/lib/app/app.db
```

## 3. Back up and verify

Restore the source's Litestream replica to a scratch path and check it: a backup that was never
restored is not a backup. Choose the restore point with `litestream ltx` and check it with
`-dry-run` first: Litestream restores only at LTX file boundaries.

```sh
litestream ltx /var/lib/v9/v9.db
litestream restore -dry-run -o /var/tmp/pre-migration.db /var/lib/v9/v9.db
litestream restore -o /var/tmp/pre-migration.db -integrity-check full /var/lib/v9/v9.db
sqlite3 /var/tmp/pre-migration.db 'PRAGMA integrity_check'
```

A local snapshot is the other safe copy of a live database, never `cp`:

```sh
sqlite3 /var/lib/v9/v9.db "VACUUM INTO '/var/tmp/pre-migration-local.db'"
```

## 4. Apply

```sh
cargo run --release -p v9-import -- --source /var/tmp/pre-migration.db --target /var/lib/app/app.db --apply
sqlite3 /var/lib/app/app.db 'PRAGMA integrity_check'
sqlite3 /var/lib/app/app.db 'PRAGMA foreign_key_check'
```

The tool prints each table's count before and after, and exits non-zero when a count breaks its
rule. Run the apply a second time: it must change nothing.

## 5. Rollback

The source was only ever read, so an aborted port is undone by discarding the target and keeping
v9 serving:

```sh
sudo systemctl stop app.service
rm -f /var/lib/app/app.db /var/lib/app/app.db-wal /var/lib/app/app.db-shm
sudo systemctl start v9.service
```

After a cutover, restore the target from its own replica at the point before the import:

```sh
litestream restore -txid <TXID> -o /var/lib/app/app.db -integrity-check full /var/lib/app/app.db
```
