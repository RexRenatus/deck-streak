# AGENTS.md

Instructions for AI coding agents working in this repository. [CLAUDE.md](CLAUDE.md) holds the
full rules; this file is the short form every agent reads first.

## Overview

DeckStreak is a Telegram Mini App and bot that gamify Anki reviews and deliver daily pre-study
readings. The backend is a Rust workspace with one crate per bounded context
([docs/CONTEXT-MAP.md](docs/CONTEXT-MAP.md)); the Mini App is SvelteKit in `web/app`; the AI
agent's public duty skills and runner are in `agent/`.

## Setup commands

```sh
rustup show
pnpm install --frozen-lockfile
```

## Build and test

```sh
bash scripts/check.sh               # the whole gate, as CI runs it
cargo nextest run --workspace       # Rust tests
pnpm -r test                        # Mini App unit tests
```

## Code style

`cargo fmt` and clippy with `-D warnings` under the workspace lint table (pedantic on); Svelte 5
runes only, `svelte-check --fail-on-warnings`. See [docs/CODE_STYLE.md](docs/CODE_STYLE.md).

## Testing

Tests are written first and recorded red then green in `docs/red-first/`. Game math is proved
against the parity oracle's goldens. See [docs/TESTING.md](docs/TESTING.md).

## Security

No secret, address, cloud id, chat id or personal data in any file: this repository is public.
Secrets reach the service as systemd credentials. See [SECURITY.md](SECURITY.md).

## Pull requests

Open pull requests against `dev` with `gh pr create --base dev`. Title them as a Conventional
Commit, add a `changelog.d/` fragment, and keep `bash scripts/check.sh` green. A pull request into
`main` from anything but this repository's `dev` fails CI by design.

Never push to `dev` or `main`; the rulesets refuse it, and every change reaches them by a pull
request. Agents never approve a workflow run from a fork, and never merge a pull request whose head repository is not `RexRenatus/deck-streak`.

## Where decisions live

- [CHARTER.md](CHARTER.md): the hard constraints.
- [docs/decisions/](docs/decisions/): every ADR.
- [docs/specs/](docs/specs/): every SPEC, with `docs/specs/planned/` for the next waves.
