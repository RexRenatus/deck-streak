---
status: proposed
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The agent's Claude Code comes from the vendor's signed package repository at one held version, installed by the private rail and never bundled with the release

## Context and Problem Statement

ADR-015 runs the agent as headless Claude Code on the host, and SPEC-043 writes the runner that
launches it, but no decision says how the command-line tool itself reaches the host. It is the
vendor's proprietary program; one native build is about 234 MB on disk (measured on the
maintainer's machine); its native installer updates it in the background by default, while the
vendor's Linux package installs need manual updates by default; and the vendor publishes a signed
apt repository, with its signing key's fingerprint in its setup documentation, and signs each
release's `manifest.json` with the same key from 2.1.89 onward. How does a pinned, verified Claude
Code reach the host, and change only on purpose?

## Decision Drivers

- The public release is AGPL (ADR-018) and carries only what this repository may redistribute; the
  tool is used under the vendor's commercial terms, not an open licence.
- What runs is what was verified, and it does not change while nobody watches (the
  subscription-proxy pack's `launch-pins-cli`; the settings template's `DISABLE_AUTOUPDATER`,
  SPEC-043 R7).
- No remote script run as root at install.
- Nothing added to the host that the agent does not need.

## Considered Options (the alternatives it was chosen against)

- The vendor's signed apt repository, its key pinned by fingerprint, one version installed and held, installed and upgraded only by the private rail: chosen, because apt verifies every package against the pinned key, package-manager installs do not update themselves by default, and the hold keeps the version still against any upgrade.
- Bundling the tool into DeckStreak's release: rejected because it is the vendor's proprietary program, used under the vendor's commercial terms rather than under a licence this repository could pass on to the public, and it would add about 234 MB to every release.
- The vendor's install script piped to a shell: rejected because it runs a remote script as root at install time and installs a build that updates itself in the background.
- A pinned native binary checked against its release's signed manifest: rejected because it proves no more than the package does (the vendor signs each release's `manifest.json`, from 2.1.89 onward, with the same key the apt repository pins), and the rail would re-implement the fetch, the signature check, the install, the hold and the removal that apt performs with that key.
- The npm package: rejected because installing it needs Node.js 22 or later and npm on the host, for the same native binary the signed apt repository delivers without them.

## Decision Outcome

Chosen option.
- **Source.** The rail adds the vendor's apt repository with its signing key stored under the
  system's keyrings and the fingerprint checked against the one the vendor documents before the key
  is trusted; the key's file and the source line are private rail files.
- **Version.** The rail installs one version and holds it (`apt-mark hold`). A new version is
  installed only after the runner's tests have passed with it on the maintainer's machine, by the
  rail, with the owner's go at gate 2 for the first install.
- **Updates off.** Package installs do not update themselves by default, the hold refuses any
  upgrade, and the settings template also sets `DISABLE_AUTOUPDATER` (SPEC-043 R7).
- **Only with the route.** The package is installed when the owner enables the AI route (gate 3,
  SPEC-063), and removed with the repository when the route is retired.

### Consequences

- Good, because the host's tool is verified against a pinned key and changes only by a deliberate,
  tested step.
- Good, because the public release stays DeckStreak's own code.
- Bad, because the host gains a third-party package source; its line is scoped to that one package
  and its key, and it is removed with the route.
- Bad, because the tool takes about 234 MB of disk, and its memory for one run is known only once
  the gate-3 rehearsal measures it (SPEC-063 R10); DeckStreak's share may then need an amendment of
  ADR-032.

### Confirmation

At gate 2 and 3: the package's version and hold, and the key's fingerprint, read back and recorded
privately; the subscription-proxy pack's `launch-pins-cli` row on the box.

## What would make this wrong

- The vendor stops publishing the signed repository, or its packages start updating themselves.
  The pinned native binary, checked against its release's signed manifest, would replace it.
- The owner chooses an API key over the subscription (ADR-054). The route's adapter changes, and
  this decision is revisited with it.

## More Information

ADR-015; ADR-018; ADR-043; ADR-054; SPEC-043; SPEC-063; the vendor's setup documentation (the apt
repository, its signing key's fingerprint, installing a specific version, update behaviour, system
requirements, the npm package's Node.js requirement, and "Binary integrity and code signing" with
its signed manifests from 2.1.89 onward), read through Context7 and on the vendor's setup page on
2026-09-28.
