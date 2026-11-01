### Fixed

- The Python mutation plan no longer caps a listing that needs more than 32 Python legs at 32
  larger legs: it refuses the listing whole, by name, before any leg starts, and a sized plan
  prints its Python legs against their ceiling of 32 (SPEC-415).
