### Fixed

- The web card frame's strip now removes the `target` attribute from every element that carries
  one, keeping each element and its `href`, and its re-parse check refuses a card in which a
  `target` comes back, so a clicked card link opens no connection. Before it, in WebKit, a card's
  clicked `target=_blank` link opened one TCP connection to the host the card names from the
  shipped card frame; the same reach on Apple platforms is inferred from WebKit's source, not
  measured. An image map's `area` and an SVG link are planted in the card suite, and an SVG link
  that asks for a new window with no `target` keeps the sandbox a channel of its own (SPEC-402,
  ADR-498, #761).
