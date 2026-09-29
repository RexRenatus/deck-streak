### Changed

- A refused owner sync names its reason by a closed set of variants at the point it is produced, so a
  reason outside the set fails to compile; the stored codes are unchanged and a test over the
  variants replaces the source scan.
- Each refusal site of the owner's sync is pinned by a test of its own, so a site that stops
  refusing fails a test.
- The cycle's own refusal of a run record it cannot read (the sync code) and of a failed recompute
  (the recompute code), and each step's reason code, are pinned by their code, so a site that refuses
  either with another code fails a test.
