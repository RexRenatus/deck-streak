### Added

- The repository declares its formal-check settings in `config/formal.json`, holding only the fields the formal checker reads; ADR-295 decides that formal checks are judged against the development branch; three tests pin the file and the reader that refuses a planted fault (#468).
