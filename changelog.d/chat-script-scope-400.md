### Security

- The page loads Telegram's Mini App script only when Telegram launched it (SPEC-400, ADR-414). Outside a launch no route asks for the script and the page adds a second policy that refuses Telegram's origin for the page's life; on a launch the start hook adds the script with no referrer to Telegram and waits for it before the first navigation (#706).
