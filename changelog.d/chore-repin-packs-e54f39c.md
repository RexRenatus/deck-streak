### Changed

- The vendored pack probes are re-pinned to phoenix-v2 e54f39c, the train that fixed four
  web-security rows which read green on code they should refuse: the token signature, the session
  cookie flags, the freshness of Telegram's launch data, and the request body limit. Two vendored
  files changed: durable-services' checks, which gain a `security-scan` row still deferred to #25,
  and rust-service's note on the body limit.
