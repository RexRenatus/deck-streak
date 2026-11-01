### Added

- The DeckStreak logo is the web app's favicon and install icons, the iPhone and iPad app's icon set, and the README's header, which shows the logo's animation below the title (#1042). Every file comes from the DeckStreak brand source byte for byte and is listed by its path, sha256 and size in `scripts/brand-files.json`, and the files keep the brand's own licence.

### Changed

- The public scrub admits a binary file only when its path, sha256 and size equal an entry of `scripts/brand-files.json`; it still searches an admitted file for every private literal and refuses every other binary as before (SPEC-579, ADR-598).
