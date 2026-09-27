# DeckStreak

DeckStreak turns your Anki reviews into XP, levels, streaks, quests and badges in a Telegram Mini
App, with daily pre-study readings written by a mentor for each subject you study. It reads the
reviews you already do in Anki, through your own Anki sync server, and scores every study day.
The bot sends a daily digest and celebrates what you earned, under one policy that knows when to
stay quiet.

> Status: early development. The repository holds the architecture, the plan and the skeleton;
> features land wave by wave (see [the plan](docs/specs/SPEC-001-campaign-prd-parity-and-waves.md)).

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Documentation](#documentation)
- [Support](#support)
- [Contributing](#contributing)
- [Source code](#source-code)
- [License](#license)

## Install

DeckStreak is a Rust workspace (the backend) and a SvelteKit Mini App (the front end). You need the
Rust toolchain pinned in `rust-toolchain.toml`, Node 24 (`.nvmrc`), pnpm 11 (pinned by
`packageManager` in `package.json`), Python 3.11 or later for the gate's probes, and
`cargo-nextest`.

```sh
git clone https://github.com/RexRenatus/deck-streak.git
cd deck-streak
rustup show                         # installs the pinned toolchain
pnpm install --frozen-lockfile      # the Mini App's dependencies
```

## Usage

Run the whole local gate, exactly as continuous integration runs it:

```sh
bash scripts/check.sh
```

Run one part while you work:

```sh
cargo nextest run --workspace       # the Rust tests
pnpm -r check                       # svelte-check, warnings fail
pnpm -r test                        # the Mini App's unit tests
python3 scripts/pack-rows.py        # every vendored pack's rows
```

Deploying DeckStreak needs a Telegram bot, an Anki sync server you run yourself, and a host with
HTTPS. [docs/OWNER-SETUP.md](docs/OWNER-SETUP.md) lists the steps. DeckStreak reads your own Anki
sync server; it does not log in to AnkiWeb.

## Documentation

- [CHARTER.md](CHARTER.md): the mission and the hard constraints.
- [ARCHITECTURE.md](ARCHITECTURE.md) and [docs/CONTEXT-MAP.md](docs/CONTEXT-MAP.md): the bounded
  contexts and how they depend on each other.
- [docs/PRD.md](docs/PRD.md) and [the campaign SPEC](docs/specs/SPEC-001-campaign-prd-parity-and-waves.md):
  what is being built, and in which wave.
- [docs/decisions/](docs/decisions/): every architecture decision and what it was chosen against.
- [AGENTS.md](AGENTS.md) and [CLAUDE.md](CLAUDE.md): instructions for AI coding agents.
- [docs/README.md](docs/README.md): the documentation index.

## Support

Ask questions in [Discussions](https://github.com/RexRenatus/deck-streak/discussions), and report
bugs with the issue forms. Report a security problem privately, as [SECURITY.md](SECURITY.md)
describes. See [SUPPORT.md](SUPPORT.md).

## Contributing

Pull requests are welcome against `dev`; `main` changes only through a release. Read
[CONTRIBUTING.md](CONTRIBUTING.md) for the branch model, the order of work (spec, schematic,
decision, tests red first, code, gate) and the commit style, and the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Source code

DeckStreak is free software under the GNU Affero General Public License. If you use it over a
network, you are offered its complete source code at
<https://github.com/RexRenatus/deck-streak>.

## License

Licensed under the GNU Affero General Public License, version 3 or (at your option) any later
version (`AGPL-3.0-or-later`). See [LICENSE](LICENSE).

---

Anki® is a registered trademark of Ankitects Pty Ltd. DeckStreak is not affiliated with or
endorsed by Anki.
