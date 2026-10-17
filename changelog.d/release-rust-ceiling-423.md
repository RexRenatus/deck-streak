### Changed

- The release pull request from `dev` into `main` may size its Rust legs past the pull request
  ceiling, up to one ceiling constant, `RUST_RELEASE_CEILING`, read from the same release mark as
  the Python ceiling. The constant stands at the pull request ceiling until an owner-signed ruling
  file sets its figure, and the tree then cites that file by path. A ceiling outside the pull
  request ceiling to `RUST_MATRIX_LEGS` is refused by name before any deal, a release listing past
  the ceiling is refused whole, by name, never capped, and the legs run in the one `mutation-rust`
  matrix. Every other run keeps `LEG_CEILING` and its refusal byte for byte (SPEC-423, ADR-437).
