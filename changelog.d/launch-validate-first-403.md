### Security

- The page asks the server to validate a Telegram launch before it loads Telegram's script, and a launch the server refuses or does not answer loads none (SPEC-403, ADR-417). The start hook sends the fragment's launch data to a new `POST /api/launch`, which reuses the session's validation and bounds and opens no session (#775).
