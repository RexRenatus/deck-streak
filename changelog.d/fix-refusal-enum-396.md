### Changed

- A refused owner sync names its reason by a closed set of variants at the point it is produced, so a
  reason outside the set fails to compile; the stored codes are unchanged and a test over the
  variants replaces the source scan.
- Each refusal site of the owner's sync is pinned by a test of its own, so a site that stops
  refusing fails a test.
- Every step whose failure refuses the owner's sync is driven through the sync by its own fault, and
  its refusal is logged under the step's name, so a step whose failure is refused by another code,
  answered, or logged as another step's fails a test.
