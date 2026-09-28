# Runbook: the host scrub, inventory to apply

Before anything of DeckStreak's first deploy changes the host, the host is inventoried and backed
up, every obsolete item is listed for the owner, and an item goes only once the owner has approved
that exact item (SPEC-060, ADR-060). The host's other services keep running throughout, and their
health is read before and after. Every step on the host waits for owner gate 2 (#161); the host's
names, the rail's lists and the snapshot's name are in the maintainer's private gate packet, and
this runbook names each step with placeholders only.

`docs/schematics/host-scrub.md` draws the files between the tools and the order of the apply's
refusals.

## What lives where

| what | where |
|---|---|
| the tools: `inventory.py`, `plan.py`, `apply.py` | `deploy/host-scrub/` in this repository; copied together, since they import each other |
| the rules' shape, with neutral values | `deploy/host-scrub/rules.example.json` |
| the rules, the inventory, each list, each approval, each apply log | the directory the private rail names, never this repository: each tool refuses an output inside it (R8) |
| the backup | one snapshot of the host's boot disk, taken from the maintainer's machine (R5) |

The rules hold the roots to read (a root marked `sizes_only` gives its totals and no entries), the
rules by class, the protected paths and the health checks. A rule has a `name`, a `class` and a
`reason`:

| class | selects | its fields |
|---|---|---|
| `backup` | the copies in `dir` matching `pattern` beyond the family's retention, and never the newest `rotation_keeps`, which the service's own rotation would make again | `dir`, `pattern`, `keep_copies` or `keep_days`, optional `rotation_keeps` |
| `environment` | the virtual environments and git worktrees under `under` that no loaded unit's commands or working directory name | `under` |
| `loose` | the files (or links) directly in `dir` matching `pattern` | `dir`, `pattern` |
| `package` | the installed package the owner lists | `package` |

The protected paths come from the private rail's protected-path list, which holds every path of
the host's other services and their data; the example protects `/etc`, `/usr`, `/boot`, the
credential socket's directory and DeckStreak's release and state directories. Nothing under a
protected path, or holding one, is ever listed, and the apply refuses it whatever an approval says.
A health check is `{"id", "argv"}`, a read command of the inventory's allow list such as
`systemctl is-active --quiet <unit>`, or `{"id", "url"}`, one GET that is green on a 2xx answer;
the private rail's health-check list covers each service on the host (R9).

## When

At gate 2 (#161), off the predecessor's schedule as SPEC-027 R2 defines it, its sync minutes
included, off every slot of the reserved-slot list the private rail provides (SPEC-053 R2), and off
DeckStreak's own job slots. Every tool runs niced: the inventory runs each of its commands under
`nice -n 19` and `ionice -c3` itself, and the steps below start each tool the same way.

Before the first step, rehearse on the maintainer's machine:
`python3 -m unittest discover -s scripts/tests -p test_host_scrub.py`. Every test builds a
synthetic host and reads nothing real.

## E1: the inventory (read-only)

Copy the three tools and the rules into a root-only temporary directory on the host, then:

```sh
sudo nice -n 19 ionice -c3 python3 inventory.py rules.json --out inventory.json
```

It reads every health check before its first read, runs only the read commands of its allow list,
and writes one file. A command outside the allow list, such as a health check that would restart a
unit, refuses the whole run before any command runs (exit 1). Read, and record privately:

- `health_before`: every check green, or stop here;
- `mounts`, and each root's `space` and totals, where a hard-linked file counts once: the free
  space against DeckStreak's need, which is its collection copy and as much again during a full
  download (SPEC-022), three releases side by side (SPEC-062 R6), the database and its daily copies
  (SPEC-064), the journal within SPEC-021's cap, and, with the AI route only, the agent's
  command-line tool (SPEC-063);
- `backups` (each family's copies and the stale ones), `loose`, `venvs`, `worktrees`, `units`
  with their settings, `packages`, `memory`, and `commands`, each with its exit status.

Copy `inventory.json` back to the private directory. Nothing on the host changed, so no rollback is
needed.

## E2: the snapshot

From the maintainer's machine, after the inventory and before the list is approved:

```sh
gcloud compute snapshots create "$SNAPSHOT" --project "$PROJECT" \
  --source-disk "$BOOT_DISK" --source-disk-zone "$ZONE"
gcloud compute snapshots describe "$SNAPSHOT" --project "$PROJECT" \
  --format='value(status,creationTimestamp)'
```

Wait for `READY`, and keep its `creationTimestamp`: the approval names the snapshot and that
instant, and the apply refuses a snapshot taken before the inventory. The snapshot is billed while
it is kept; it is kept until the owner releases it after W2's first week, then deleted with
`gcloud compute snapshots delete "$SNAPSHOT" --project "$PROJECT"`.

## E3: the list, and the owner's approval

On the host, since the plan reads each candidate's content to digest it:

```sh
sudo nice -n 19 ionice -c3 python3 plan.py inventory.json rules.json --out list.json
```

The plan refuses rules other than the ones the inventory read (run the inventory again), deletes
nothing and writes only the list. Each item has an `id`, its `class`, its `path` (or `package`), the
`rule` and `reason` that selected it, the `bytes` it frees when the whole list is applied (a file
whose other links survive frees nothing) and its `digest`; `skipped` names each candidate left out,
with why. Send the owner the list's `digest` and a table of the items. The owner approves item ids,
one by one; write the approval beside the list:

```json
{
  "list_digest": "<the list's digest>",
  "items": ["<an approved id>", "<another>"],
  "approver": "<who approved>",
  "date": "<the approval's date, YYYY-MM-DD>",
  "snapshot": {"name": "<the snapshot>", "taken_at": "<its creationTimestamp>"}
}
```

Nothing changed, so no rollback is needed.

## E4: the apply

Read the health checks and the free space (`df -h /`) first. Then, on the host, a dry run, and the
apply:

```sh
sudo python3 apply.py list.json approval.json --rules rules.json --log apply-dry.json
sudo nice -n 19 ionice -c3 python3 apply.py list.json approval.json --rules rules.json \
  --log apply.json --apply
```

Without `--apply` nothing is deleted: the run checks everything and logs what would go. Every
check stands before the first deletion, and one failure refuses the whole run, naming the item and
the reason: a list whose digest does not match its content; no approval, or one without the list's
digest, the approver, the date or ids the list holds; no snapshot, or one taken before the
inventory; an item under a protected path or holding one; an item reached through a symbolic link;
an item whose digest changed since the list was made; a package that `dpkg --dry-run --remove`
would not remove alone. A file or link is unlinked, never its target; a directory is removed
without following a link inside it; a package is removed with `dpkg --remove`, which keeps its
configuration files.

| exit | meaning | what to do |
|---|---|---|
| 0 | done, or a clean dry run | read the log; the health checks and `df -h /` again |
| 1 | refused: nothing was deleted | the reason names the item; make a new list if the host changed |
| 2 | a usage error | a file is missing, malformed, or inside this repository |
| 3 | a deletion failed part way | the log names what went and what failed; stop, and restore if a service needs it |
| 4 | a health check is red after the apply | the scrub stops: the log's `turned` names each check that was green before |

Copy the log back to the private directory, and remove the temporary directory.

## Rollback: restore an item from the snapshot

The snapshot restores any item without room on the host. From the maintainer's machine:

```sh
gcloud compute disks create "$RESTORE_DISK" --project "$PROJECT" --zone "$ZONE" \
  --source-snapshot "$SNAPSHOT"
gcloud compute instances attach-disk "$INSTANCE" --project "$PROJECT" --zone "$ZONE" \
  --disk "$RESTORE_DISK" --mode ro
```

On the host, mount it read-only and copy the item back with its owner, mode and times:

```sh
sudo mkdir -p "$MOUNT" && sudo mount -o ro,noload "$RESTORE_DEVICE" "$MOUNT"
sudo cp -a "$MOUNT$ITEM" "$ITEM"
sudo umount "$MOUNT"
```

Then detach the disk and delete it:

```sh
gcloud compute instances detach-disk "$INSTANCE" --project "$PROJECT" --zone "$ZONE" \
  --disk "$RESTORE_DISK"
gcloud compute disks delete "$RESTORE_DISK" --project "$PROJECT" --zone "$ZONE"
```

A removed package is installed again with the package manager, at the version the list recorded.
Read the health checks again after any restore.
