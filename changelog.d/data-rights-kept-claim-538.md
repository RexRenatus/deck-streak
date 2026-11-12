### Changed

- The data-rights symmetry check now seeds a queued notification abandoned with its claim kept,
  beside the rows that hold no claim, and a new test holds that the export carries that claim by its
  value; a mutation row pins the export of the claim, so an export that wrote every claim as null
  now fails (SPEC-041, #538).
