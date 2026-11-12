### Changed

- Every web screen now paints in Tide outside Telegram, in a dark and a light mode. The token file
  gains a card, a muted-text and a border value for each mode; every pair of text and its
  background meets 4.5:1, and every border and focus ring 3:1, in both modes; and every card
  carries a border. Three screen classes that named a missing role now name an existing one, and
  the iPhone and iPad app asset catalog gains three more colour sets, `TideCard`, `TideMuted` and
  `TideBorder`, copied from the token file. Inside Telegram the chat theme still paints every
  screen (#1044; SPEC-593).

### Added

- A colour mode for the web app: a new settings screen offers system, light or dark, kept on the
  device. With system, or with nothing chosen, the page is light only when the browser asks for
  light, and dark otherwise. The stylesheet paints that mode from the first frame with no script,
  and a stored light or dark applies once the app has loaded (a maintainer checks the first frames
  in a real browser) (#1045; SPEC-593).
