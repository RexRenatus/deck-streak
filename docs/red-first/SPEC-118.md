# Red-first record: SPEC-118

This record is V1a's, the first of SPEC-118's two pull requests. It records the criteria this pull
request delivers (A1 to A6 and A15 to A23); V1b adds the lines of A7 to A14 when it moves them back
into the acceptance fence (SPEC-118 section 3c).

The order of work: the SPEC moved out of `docs/specs/planned/` as a pure rename, then its status and
section 3c; the CaptureOnce model with its witness, checked before any vault code; the registry and
the `inbox_capture_stub` golden; the vault's tests beside stubs that compile and answer nothing;
their implementation; then the coordination use case, the route, the Mini App screen and the
wiring.

Each criterion is run at its red commit, selecting its own test, and fails by assertion, not by a
compile error, a missing fixture or an empty selection.
