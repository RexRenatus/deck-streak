The owner's decision, carried by the owner's signature on the commit that adds this file: Rex, DeckStreak's own memory model, enters this repository and is DeckStreak's only scheduler from the Nov 1 release, under the release gates below; if any gate fails, the release is delayed, not shipped with FSRS.

# OWNER RULING 2026-10-18: Rex enters DeckStreak

It lands with the amendment to ADR-339, `docs/decisions/ADR-339-amendment-rex-may-set-due-dates.md`, in the same signed commit. Until that commit is signed, Rex stays out of this repository.

## What the owner rules

1. **Rex enters this repository.** These parts move from Rex's private research track into it:
   - Rex's Rust core: one crate compiled for iOS, WASM and the server, bit-identical across them.
   - Its curve head.
   - The Rex package formats and the Anki to Rex and Rex to Anki converters.
   - The in-app AI card-restructure feature and its personas.

   The trained weights enter only once condition 3(c) holds.
2. **Rex is DeckStreak's only scheduler from the Nov 1 release.** FSRS is removed from live scheduling. Its baselines stay in the private research track as the yardstick.
3. **The release gates.** Rex ships only if all of these hold at the Oct 28 go/no-go:
   - (a) **Prediction-ready:** Rex beats FSRS-7 on LogLoss and RMSE(bins) on full DEV, holds on the sealed set, and no scorecard cell is worse.
   - (b) **Scheduling-ready:** (a) holds, and Rex also wins the offline simulator.
   - (c) **Data permission:** written permission from Anki's maintainers to ship weights trained on anki-revlogs-10k and FSRS-Anki-20k is on file.

   If any gate fails, the release is delayed, not shipped with FSRS.
4. **What stays private.** These stay in the private research track and are never committed to this repository:
   - The licensed review-log datasets, which are never redistributed and never leave the private training environment.
   - The research cycle, its fit results, its programs and its clean-room records.

   This repository carries the shipped model and a public evaluation summary of aggregate figures only.
5. **Licensing.** The repository is dual-licensed: AGPL-3.0-or-later for everyone, plus a commercial license the owner may grant, because the owner holds the copyright to DeckStreak's own code.
   - Outside contributions come in under a contributor license agreement that lets the owner relicense them.
   - Builds that link Anki's engine (the server's ingest crate) stay AGPL; the commercial license covers DeckStreak's own code.
6. **Unchanged.** These still bind:
   - Palace data is never used for training.
   - It travels end-to-end encrypted.
   - Rex runs on the device, with the server as a relay.
   - A personal adapter learns on the device from the user's own history only.
   - Central training uses public data only. Structure sharing across users is a design the owner signs separately before anything is shared.
