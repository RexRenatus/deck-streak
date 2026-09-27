# Code Style

## Formatting

```sh
cargo fmt --all
pnpm -r format
```

## Linting

```sh
cargo clippy --workspace --all-targets --locked -- -D warnings
pnpm -r check
```

The workspace lint table turns on clippy's pedantic group; a lint is allowed only with its reason
beside it in `Cargo.toml`.

## Naming

One concept, one name ([LEXICON.md](LEXICON.md)). Types are named for their context's language;
tests are named for the behaviour they pin, as a sentence.

## Error handling

Library crates return their own `thiserror` enums with the source chain kept; only the binary uses
`anyhow`. An axum service maps its error type to a response in one place.

## Comments and documentation

Public items have doc comments that say why, not what. No `TODO`, `FIXME`, `HACK` or `XXX`.

## Commits

Conventional Commits, `type(scope): description`, no attribution trailers.
