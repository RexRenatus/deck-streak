# Contributing to deck-streak

Thank you for helping. DeckStreak is built spec first and test first; this page says how a change
travels from an idea to a release.

## Branch model

- `main` is protected: it changes only through a release pull request from `dev`, merged with a
  merge commit, and every release is tagged there. It is the branch visitors see.
- `dev` receives every other pull request. Open every pull request against `dev`:
  `gh pr create --base dev`.
- A pull request into `main` whose head is not this repository's `dev` fails the `base-is-dev`
  check by design, whatever a fork names its branch.
- Nothing is merged from `main` back into `dev`, and a hotfix is a pull request into `dev` like any
  other change ([RELEASING.md](RELEASING.md)).
- Nobody force-pushes or deletes either branch; required checks, not reviews, are the bar.

## Proposing a change

1. Open or pick an issue. A feature issue names its SPEC and its acceptance criteria.
2. Fork the repository and branch from `dev`.
3. Follow the order of work in [CLAUDE.md](CLAUDE.md): SPEC, schematic, ADR, tests red first,
   implementation, gate.
4. Open a pull request against `dev` with the template filled in.

Outside contributions are welcome; for anything larger than a fix, open an issue first so the
change can be specified before it is written.

### Pull requests from forks

A workflow run from a fork waits for a maintainer's approval, and the required checks count only
when GitHub Actions produced them. An accepted outside contribution is re-landed by the maintainer
from a branch of this repository, keeping its author's credit; the fork's pull request is then
closed with a link to the one that landed. Agents never approve a workflow run from a fork, and never merge a pull request whose head repository is not `RexRenatus/deck-streak`.

## Running the checks

```sh
bash scripts/check.sh
```

It is the same script CI runs: formatting, clippy with warnings denied, the Rust tests and doctests,
the Mini App's checks and tests, the parity oracle, the public scrub and the secret scan. The packs
are judged on the maintainer's box, whose verdict a pull request shows as `box/packs`.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description`, for
example `feat(readings): grant XP once per reading`. No attribution trailers.

## Changelog

Every pull request adds one fragment under `changelog.d/` instead of editing `CHANGELOG.md`; see
[changelog.d/README.md](changelog.d/README.md). The release compiles them.

## Licensing of contributions

DeckStreak is licensed AGPL-3.0-or-later. By contributing you agree that your contribution is
licensed under the same terms, including the licence's network-use clause.
