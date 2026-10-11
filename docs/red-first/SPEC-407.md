# Red-first record: SPEC-407

SPEC-407 (R1 to R9, A1 to A7). The SPEC, ADR-421, the schematic's amendment, the stub host and the
tests of A1 to A6 were committed first, over a host that carries the card document unchanged and a
host policy that is empty, and each of the six read red for its own criterion. The host, its policy
and the `CardFrame` that sets it follow; the four study reads and the two card-frame reads go
through the host. A1 to A5 are Vitest tests; A6 is the Playwright case read in `card-sandbox`,
beside the Vitest test that pins it. A7 is the planted suite as it stood at `dev`, which the fix
leaves passing. Each red below is quoted from the run at its commit.

Disclosure for the fix commit 7bd7f1e: it redirects two reads in `card-frame.test.ts` (the opening of the
card document and the body parse) through the host's one frame, with every expected value kept, and
it adds the host's own killers and the `hosted` import to `frame-host.test.ts` beside A1 and A2, which
stay as written. No assertion is removed or loosened.

```red-first
A1: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: AssertionError: expected  to have a length of 1 but got +0 (the host holds no policy meta element)
A1: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A2: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: AssertionError: expected { srcdoc: '<p>a\rb</p>' } to deeply equal { refused: 'escaped' } (a carriage return in the card was kept)
A2: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A3: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: AssertionError: expected [] to deeply equal [ 'img-src', 'script-src', ...(2) ] (the host policy is empty)
A3: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A4: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: AssertionError: expected 'default-src \'none\'; img-src data:; ...' to be 'img-src data:; script-src \'none\'; o...' (the frame's document is the frame document, not a host)
A4: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A5: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: AssertionError: expected { ...(1) } to deeply equal { ...(2) } (the census names one sink, not two)
A5: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A6: red at 26e79ce5be8d6fe16b4d40c3c32e13bed5c2b7de: Error: srcset opened under the host policy alone in chromium; Error: srcset opened under the host policy alone in webkit; Error: srcset opened under the host policy alone in firefox (card-sandbox job 114251622163, the case "the host policy alone holds the image-set forms in every engine")
A6: green at 7bd7f1eb8aef4cc27370180415e86d96763b7b02
A7: not red: the eight failing jobs 114200425555, 114216666394, 114215917120, 114224746266, 114225289308, 114225399072, 114225529870, 114227082352 and the passing job 114191481550 are the observation that opened #787, never red-first evidence
```
