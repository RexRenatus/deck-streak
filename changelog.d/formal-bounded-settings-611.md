### Changed

- The formal settings test holds `k`, each budget and each per-entry budget to the range the
  formal checker reads, and refuses a per-entry budget whose key names no TLA+ entry, so a
  settings file the checker would refuse no longer passes it (SPEC-611, #802).
