# Schematic: the host scrub, inventory to apply

Kind: data flow, and the order of the apply's refusals. Read at DeckStreak `dev` 24b04d3 (SPEC-060,
ADR-060, ADR-010, ADR-059). Built by SPEC-060. `docs/schematics/first-deploy-and-gates.md` draws the
scrub at the level of the owner's gates (gate 2, #161); this page draws the three tools, the files
they pass between them, and every check that stands before a deletion. The rules, the inventory,
the list, the approval and the log live in a directory the private rail names, never in this
repository (R8); the repository holds the tools and `deploy/host-scrub/rules.example.json`.

| step | reads | where |
|---|---|---|
| the inventory reads, and changes nothing | its own allow list of read commands, each under `nice` and `ionice -c3` | SPEC-060 R1, R2 |
| the rules | roots, backup retention, protected paths, packages, health checks | SPEC-060 R3, R7, R9 |
| the list | one item per path (or per listed package), each with its reason and digest | SPEC-060 R4 |
| the backup | one boot-disk snapshot, taken after the inventory, named by the approval | SPEC-060 R5 |
| the apply | all or nothing, before its first deletion | SPEC-060 R6, R7 |
| the health checks | read before the inventory and after each apply | SPEC-060 R9 |

## 1. The files between the tools

```mermaid
flowchart LR
  rules[("rules.json: roots, rules, protected paths, health checks")] --> inv
  inv["inventory.py RULES --out"] --> inventory[("inventory.json: space, sizes, stale copies, environments, units, packages, commands, health before")]
  inventory --> snap["boot-disk snapshot, from the maintainer's machine"]
  inventory --> plan["plan.py INVENTORY RULES --out"]
  rules --> plan
  plan --> list[("list.json: items with reason, bytes and digest; the inventory's instant; its own digest")]
  list --> owner{{"the owner approves item ids (gate 2)"}}
  snap --> owner
  owner --> approval[("approval.json: the list's digest, item ids, approver, date, the snapshot and its instant")]
  list --> apply["apply.py LIST APPROVAL --rules --log"]
  approval --> apply
  rules --> apply
  apply --> log[("the apply log: each check, each deletion, health before and after")]
```

The plan reads each candidate's content to digest it, so it runs where the candidates are; the
inventory and the plan write their one output each, and the apply writes only its log until
`--apply` is given.

## 2. The apply's checks, in order

Every check below runs before the first deletion. One failure refuses the whole run, names the item
and the reason, and deletes nothing (R6, R7). The rules are the ones the inventory read, bound by
their digest, which is taken over the very bytes the apply parsed them from in one read (A10), and
the health checks run through the inventory's read allow list alone; the changing commands the
apply admits run only for a listed package's item.

```mermaid
flowchart TD
  start(["apply.py LIST APPROVAL"]) --> reads{"every health check a read command of the allow list"}
  reads -->|"no"| refuse(["refused: nothing deleted, the reason named"])
  reads --> selfd{"the list's own digest recomputed"}
  selfd -->|"differs"| refuse
  selfd --> bound{"the rules are the ones the inventory read (the list names their digest)"}
  bound -->|"no"| refuse
  bound --> present{"an approval is there"}
  present -->|"no"| refuse
  present --> carries{"it carries the list's digest"}
  carries -->|"no"| refuse
  carries --> names{"approver, date, and item ids the list holds"}
  names -->|"no"| refuse
  names --> snapshot{"a snapshot, taken after the inventory and not dated after the apply's clock"}
  snapshot -->|"none, taken before, or dated later"| refuse
  snapshot --> clock{"the inventory recorded a synchronised clock, and the clock reads synchronised now"}
  clock -->|"no"| refuse
  clock --> each["each approved item"]
  each --> canon{"its path absolute and canonical"}
  canon -->|"no: named by its id"| refuse
  canon --> prot{"under a protected path, or holding one"}
  prot -->|"yes"| refuse
  prot --> link{"reached through a symbolic link"}
  link -->|"yes"| refuse
  link --> dev{"every entry under it on the item's own device"}
  dev -->|"no"| refuse
  dev --> dig{"its digest, computed now, equals the listed one"}
  dig -->|"no"| refuse
  dig --> pkg{"a listed package removes alone (a dry removal)"}
  pkg -->|"no"| refuse
  pkg --> mode{"--apply given"}
  mode -->|"no"| dry(["dry run: what would go, and the bytes, in the log"])
  mode -->|"yes"| before["health checks read"] --> again{"each item read again, through directories opened without following a link: the entry, and its digest measured again, still what its checks read"}
  again -->|"no"| stopped(["stopped part way: the log names what went"])
  again -->|"yes"| del["delete it: a file or link is unlinked, a directory removed without following a link, a package removed"]
  del -->|"the next item"| again
  del -->|"after the last"| after["health checks read again"]
  after -->|"any red"| stop(["the scrub stops: the log names each check, and which turned"])
  after -->|"all green"| done(["done: the log holds every deletion and its bytes"])
```

The health read before the first deletion lies between the checks and the deletions, so each
deletion reads its item again, digest included, immediately before it deletes (SPEC-060 §7). The
inventory refuses the same clock reading before it writes anything.

## 3. The digest

A file's digest is SHA-256 over one line, `path NUL size NUL mtime_ns NUL mode NUL content`, where
`mode` is `st_mode` in octal and `content` is the hex SHA-256 of the file's bytes (of the link's
target for a symbolic link, empty for anything else). A directory's digest is SHA-256 over the
sorted lines of every entry under it, the directory itself included, joined by newlines. A listed
package's digest is SHA-256 over its name, architecture, version and state. The list's own digest
is SHA-256 over the list written as canonical JSON without its `digest` field, and the approval
carries it.
