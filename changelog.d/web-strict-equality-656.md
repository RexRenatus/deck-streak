### Added

- A text census over the web client's TypeScript and Svelte sources refuses nothing yet and
  reports at warn: each `==` or `!=` that is not strict (none listed), and each place at the
  client's boundary that spells an absent value `undefined` where the client's answer types spell
  it `null` (23 listed today). It prints what it examined and is VOID on an empty population
  (SPEC-656).
