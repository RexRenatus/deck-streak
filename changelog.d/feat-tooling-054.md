### Fixed

- `scripts/public-scrub.py --subject` takes a file as its own subject, refuses by name a subject
  that does not exist, and reads VOID for a subject that examined nothing, even beside a tree it
  did examine.
- The scrub's email rule passes a systemd unit instance name, such as `getty@tty1.service`, and
  still refuses every address, including one whose domain only holds a unit type's word.
- A deny list the scrub cannot read stops it with exit 2 and the reason, never a traceback that
  read as a finding.
- `scripts/box-packs.sh` fails an expected red or a pending entry whose issue is closed, naming
  the pack, the row and the issue, and reads VOID when `gh` cannot tell it an issue's state.
- The vault rails guard bounds its `cargo run` at 900 s, and fails by name when the bound expires.

### Changed

- The deny lists are composed once, in `public-scrub.py`'s `rules()`, which
  `scripts/vendor-packs.py` calls; the vendoring's output is unchanged.

### Added

- The parity oracle's README documents the lossless `{day:N}` token that a text golden uses for a
  day, with an example that its tests round-trip.
