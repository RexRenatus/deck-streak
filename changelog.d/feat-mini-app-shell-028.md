### Added

- The Mini App shell: one typed wrapper around Telegram's script, which reads the launch once, gates
  each newer method by the client's version and says so when the page runs outside Telegram;
  startapp links that open a screen only through a closed table, with anything else opening Today;
  Today with the server's study day; an About screen linking the privacy policy and the source
  code; and one API client that opens a session with Telegram's signed launch data and then sends
  only the session cookie.
- Design tokens in the DTCG format, compiled at build into Telegram's theme variables with
  fallbacks, with every text colour proved to meet WCAG 2.2 AA in both of Telegram's default
  palettes, and an axe audit of every screen in both.

### Security

- The Mini App's page carries its own Content-Security-Policy, which admits scripts only from
  itself, from Telegram and by the hash of each inline script the build generates, and it sends no
  referrer to another origin.
