### Changed

- `config/formal.json` sets `tlc_slot.capacity` to 4, equal to the formal checker's own setting, and keeps `wait_seconds` at 1800, because the checker's slot directory is shared and it refuses a settings file whose capacity differs from its own; SPEC-295 and ADR-295 are amended, the settings test pins the value and a mutation row holds it (#516).
