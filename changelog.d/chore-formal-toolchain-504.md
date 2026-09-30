### Changed

- `config/formal.json` names the formal checker's toolchain by its identity, so a TLA+ or Lean check uses the checker's own tool pin, refuses a checker built with another pin as drift, and no pin file is committed; SPEC-295 and ADR-295 are amended and the settings test pins the field and refuses a malformed digest (#504).
