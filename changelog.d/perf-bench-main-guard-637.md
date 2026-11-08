### Fixed

- `scripts/tests/test_perf_benches.py` ends with its `unittest.main()` guard, so a script run of
  the module reaches every class, the web bench bound test included; a test holds the guard as the
  last statement of the module (SPEC-637, ADR-653).
