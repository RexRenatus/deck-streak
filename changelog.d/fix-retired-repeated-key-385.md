### Fixed

- `mutation_rows.py retired` reads the retirement list through the same parser as the band files,
  so a key repeated in `scripts/mutation-rows.retired.json`, at its top or inside one entry, is
  refused by name with exit 2 instead of keeping the last value (SPEC-122, #385).
