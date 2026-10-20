### Added

- Four state-machine lints, each a script test of its own over one register,
  `config/fsm-lints.json`, start at warn. FSM-CHECK-LIST runs every migration into an in-memory
  SQLite database and compares each stored state column's CHECK list with its enum's spellings;
  FSM-WILDCARD names a catch-all arm in a match on a state enum; FSM-UNREACHABLE names a match arm
  whose value is a panicking macro; and FSM-UNCOVERED names a state enum that no formal entry
  anchors. Each prints one summary line and one line per hit in the `hygiene` log, and fails only
  as VOID, on a register it refuses, or at fail on a new or stale hit. The sites the tree holds
  today are listed, and no product code, migration or formal entry changes (SPEC-433, ADR-447).
