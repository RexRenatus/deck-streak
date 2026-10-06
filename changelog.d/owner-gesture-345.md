### Added

- The engine core's six exempt writes (forget a card, set its due date, delete a preset, change a
  note's type, delete a card, delete a note) run only behind an owner-gesture token (SPEC-345, part
  2): each client's adapter has one exempt entry that builds the token from a tap, and the core
  runs the write only when its request names the tapped card, note or preset alone. A containment
  test refuses any other caller of the token, its entry or the engine's write names.
