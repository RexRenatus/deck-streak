# Red-first record: SPEC-565

Commit 1 (3d98a476) installed the SPEC, ADR-583, the guard, its tests and fixtures, the audit's
guard line and the migration hold guard over 4e600dbb, with `Cargo.toml`, `Cargo.lock` and
`deny.toml` unchanged. At that commit the workspace's lock still held block-buffer 0.10.4 and
digest 0.10.7, through the registry's sqlx 0.9.0 and the engine at a078820, and the engine fork's
lock at a078820 held block-buffer 0.12.0 and digest 0.10.7. A1 and A3 fail there by assertion, for
the advisory's reason, while the guard's fixture legs (A2) and the non-vacuity control (A4) pass.
A5's two tests were written first and run against 4e600dbb's `audit-sync-server.sh`, before the
guard line existed: both fail by assertion, and with the line added at commit 1 the file passes
whole (`Ran 5 tests`, OK). A6's goldens were taken at 4e600dbb, and a planted wrong golden reads
red before the true one reads green.

Commit 2 pins the engine fork's dca72f35 and the sqlx fork's 9e828247, and cargo writes the lock:
block-buffer 0.10.4, digest 0.10.7 and sha2 0.10.9 leave it, and the fork's lock at dca72f35 holds
block-buffer 0.12.1 and digest 0.11 alone. A6b's red was taken in commit 2's tree with `deny.toml`
edited and `test_engine_pin.py`, `Cargo.toml` and `Cargo.lock` still as commit 1 left them. At the
commit that adds this record every test of the fence passes, A6 on the sqlx fork's crates.

```red-first
A1: red at 3d98a476: AssertionError: 1 != 0 : lock-digest-line: Cargo.lock: 646 packages; examined 2 block-buffer, 2 digest; block-buffer below 0.12.1: 1 of 2: 0.10.4 <- digest 0.10.7
A1: green at the commit that adds this record
A2: not red: controls any guard keeps; the fixture legs pass at 3d98a476
A3: red at 3d98a476: AssertionError: 1 != 0 : lock-digest-line: a078820's Cargo.lock: 596 packages; examined 2 block-buffer, 2 digest
A3: green at the commit that adds this record
A4: not red: the non-vacuity control; it passes at 3d98a476
A5: red with 4e600dbb's audit-sync-server.sh: AssertionError: 0 != 1 : a RED fork lock must end the audit with exit 1; AssertionError: 0 != 2 : a missing fork lock must end the audit with exit 2
A5: green at 3d98a476
A6: not red: a hold guard; a planted wrong golden reads red: panicked at crates/kernel/tests/migration_checksums.rs:62:5: assertion `left == right` failed
A6: green at 3d98a476
A6b: red with deny.toml edited and the test unchanged: allow-git is [...], not exactly the fork, rust-url, fsrs-rs and web-push-native
A6b: green at the commit that adds this record
```
