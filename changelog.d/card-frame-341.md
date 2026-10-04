### Security

- A card face renders on the web only in a sandboxed `srcdoc` frame that runs no script, carries
  its own policy admitting `data:` images, media and fonts alone, and holds none of the card's
  `link`, `meta`, `base` or `template` elements; a card whose markup would escape that document is
  refused (SPEC-341, ADR-352). The page policy gains `frame-src 'none'`, so a card frame cannot
  navigate itself (SEC01-F13).
- A planted suite, the `card-sandbox` job, opens one planted card for every channel the
  schematic names in Chromium and WebKit: each reaches a listener the test owns from a reference
  frame with every layer off, and nothing from the shipped frame (SEC01-F14 and SEC01-F15, web).
