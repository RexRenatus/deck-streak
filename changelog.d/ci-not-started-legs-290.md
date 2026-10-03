### Changed

- A mutation leg with nothing to examine is no longer started: `mutation-rust` runs only when the plan's listing holds a mutant, and `mutation-rows` only when the plan selects a row or its retirement check is due. The verdict judges each skipped leg against the listing and refuses, by name, a listed leg that is missing and any examined sum that differs from the listing; `ci` admits a skip from those two legs alone (#435).
