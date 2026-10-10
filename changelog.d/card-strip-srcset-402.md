### Fixed

- The web card frame's strip now removes the `srcset` attribute from every element that carries
  one, keeping each element and its `src`, and its re-parse check refuses a card in which a `srcset`
  comes back. The image-set forms are planted as their own `srcset` card in the card suite
  (SPEC-402, ADR-416, #771).
