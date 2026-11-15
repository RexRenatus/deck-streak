### Added

- A transition table for each product state enum, kept as data in `config/fsm-tables.json`, and
  five lints, each its own test beside the state-machine lints: FSM-TABLE, a
  product state enum that no table names; FSM-REACH, a state no walk from the initial set
  reaches; FSM-DEAD-END, a reached state outside the terminal set that no row leaves;
  FSM-TERMINAL, a reached terminal that a row leaves; and FSM-DETERMINISTIC, one state moved two
  ways by one event. Every rule lands at warn with no hit listed, prints one summary line and one
  line per hit, and fails as VOID when it examined nothing, when the tree has no `crates/` or no
  register, or when a table names a file, an enum or a state that is absent (SPEC-434, ADR-448).
