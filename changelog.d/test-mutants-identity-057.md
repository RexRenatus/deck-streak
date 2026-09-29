### Added

- Tests hold the identity crate to SPEC-057: every cargo-mutants mutant of `deck-streak-identity`
  is now killed or recorded equivalent. The session cookies, the presented id, the ending of a
  session, the reading of an id, the `Debug` of the owner, the key, the token and the store, the
  owner credential's sign and zero, and an `auth_date` with a sign each gained a test that fails
  when the behaviour changes.
- Three mutants that join a byte's two hex digits with `^` instead of `|` are recorded equivalent
  in `scripts/mutation-equivalent.d/deck-streak-identity.json`, each with the argument that the two
  halves share no bit (SPEC-057 R5).
