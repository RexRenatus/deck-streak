### Added

- The iPhone and iPad app's shell (SPEC-347, part 2 of 2, #625): a project of its own beside the
  harness's, whose one split view lists the decks of the engine's own collection by name and opens
  an account sheet. Signing in sends the configured user, the configured endpoint and the typed
  password through the engine's login call; the host key it answers with is kept in one Keychain
  item on this device only, the password is kept nowhere, and signing out deletes the item. The
  sync settings are build settings whose committed values are reserved placeholders. The client's
  codec gains the login request and two decoders, pinned by literal bytes and by mutants. Two
  censuses hold the app's tree: every Swift file under `ios/` is registered with a role, doors and
  an exact count of its decisions, and the app's property lists, settings and credential store keep
  the seam. The Apple job body's `harness` job gains the app's tests on both simulators and an
  unsigned archive for a device, whose required-reason imports are checked against its privacy
  manifest. ADR-358 D9 and D10 record the two choices this part adds.
