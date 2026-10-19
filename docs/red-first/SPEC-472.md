# Red-first record: SPEC-472

SPEC-472 (R1 to R7, A1 and A2). The bound is already on dev: `check_caption` counts a caption in
UTF-16 units and both the photo and the prepared share ask it before any transport call. So the
delivery changes no production line, and neither criterion can be red at the base. Both are
mutation coverage: each records `not red` and names the hand row that is its red. The SPEC, ADR-489
and the schematic were committed first; the Lean entry, its writer, its vectors and the test of A1
and A2 were committed together in `fa149429156462621e70cad223bc3851fdb2323b`, the covers were
stamped in `b6e815c85d9144c28c0666d889a3be27a2d2179f`, the vectors were rewritten under the stamped
digest in `fd22fef405f3fd64b2b1e57bdccc02b1b4f7adc2`, and the rows were committed in
`c6796b80d97af23a6ddee356581b69f647ed630c`. Every reading below was taken locally.

```text
A1: cargo test -p deck-streak-notifications --test formal_vectors_caption_bound -- --exact the_caption_bound_answers_every_lean_vector
A2: cargo test -p deck-streak-notifications --test formal_vectors_caption_bound -- --exact the_lean_vectors_hold_each_caption_a_broken_count_misjudges
```

```red-first
A1: not red: pins the base's caption bound against every vector of the Lean port; its red is hand row S47200, killed by A1
A2: not red: pins the classes the vectors hold; its red is hand row S47206, killed by A2
```

Each fence line selected one test and passed: A1 printed `examined 55 caption vectors, 37 taken
and 18 refused`, and A2 printed `examined 55 caption vectors`, its five classes reading 6 captions
a count of characters misjudges, 31 a count of UTF-8 bytes misjudges, 18 at exactly the limit, 12
whose last character is past the basic plane, and 9 distinct characters.

## The plants

Four mutants of the S13200 band, each installed alone from its row's own find and replacement in
the committed tree, with A1 run alone by its fence line. Each turned A1 red on its one misjudged
assertion, quoted below after the plant. Each source was then restored from its saved bytes, its
sha256 compared equal to the one recorded before the plant, the file was touched, the crate was
rebuilt and A1 passed again before the next plant, and the work tree named no changed file under
`crates`.

```text
plant: S13212 counts the caption by its characters, caption.chars().count() > MAX_CAPTION_UTF16
6 of 55 vectors are misjudged, the first line 43: {"rule":"caption","runs":[[65536,512],[97,1]],"takes":false,"units":1025}: the code answers (1025, Ok(()), Ok(()), Some(Ok(())), 1), the port (1025, Err(Caption), Err(Caption), Some(Err(Caption)), 0)
plant: S13216 refuses a caption at the limit, encode_utf16().count() >= MAX_CAPTION_UTF16
18 of 55 vectors are misjudged, the first line 5: {"rule":"caption","runs":[[97,1024]],"takes":true,"units":1024}: the code answers (1024, Err(Caption), Err(Caption), Some(Err(Caption)), 0), the port (1024, Ok(()), Ok(()), Some(Ok(())), 1)
plant: S13213 has the share ask the bound of the empty caption, check_caption("")
18 of 55 vectors are misjudged, the first line 7: {"rule":"caption","runs":[[97,1025]],"takes":false,"units":1025}: the code answers (1025, Err(Caption), Err(Caption), Some(Ok(())), 1), the port (1025, Err(Caption), Err(Caption), Some(Err(Caption)), 0)
plant: S13215 has the photo ask the bound of the empty caption, check_caption("")?
18 of 55 vectors are misjudged, the first line 7: {"rule":"caption","runs":[[97,1025]],"takes":false,"units":1025}: the code answers (1025, Err(Caption), Ok(()), Some(Err(Caption)), 0), the port (1025, Err(Caption), Err(Caption), Some(Err(Caption)), 0)
```

S13212 takes the six captions of 1,025 units that hold 1,024 characters or fewer; S13216 refuses
the 18 captions at the limit; under S13213 the share answers `Ready` after one transport call for
the 18 the port refuses; and under S13215 the photo is built for those 18.

## The rows

Each row of `S47200-S47299` was proved by hand on the committed vectors: the replacement installed
once, the killer run alone by `--exact` (one test), the target's other test run the same way, then
the vectors restored from their saved bytes, their sha256 compared equal to the one recorded before
the row, the vectors and the test file touched, and the test target rebuilt before the next row.

```text
S47200 KILLED by A1: 1 of 55 vectors are misjudged, the first line 48: {"rule":"caption","runs":[[97,1022],[128512,1]],"takes":false,"units":1024}: the code answers (1024, Ok(()), Ok(()), Some(Ok(())), 1), the port (1024, Err(Caption), Err(Caption), Some(Err(Caption)), 0)
S47200 A2 failed too: left: (7, 30, 18, 12, 9) right: (6, 31, 18, 12, 9)
S47201 KILLED by A1: 1 of 55 vectors are misjudged, the first line 56: {"rule":"caption","runs":[[97,1023],[1114111,1]],"takes":true,"units":1025}: the code answers (1025, Err(Caption), Err(Caption), Some(Err(Caption)), 0), the port (1025, Ok(()), Ok(()), Some(Ok(())), 1)
S47201 A2 failed too: left: (5, 32, 18, 12, 9) right: (6, 31, 18, 12, 9)
S47202 KILLED by A1: 1 of 55 vectors are misjudged, the first line 49: {"rule":"caption","runs":[[128512,512],[97,1]],"takes":false,"units":513}: the code answers (1025, Err(Caption), Err(Caption), Some(Err(Caption)), 0), the port (513, Err(Caption), Err(Caption), Some(Err(Caption)), 0)
S47202 A2 passed
S47203 KILLED by A1: 1 of 55 vectors are misjudged, the first line 35: {"rule":"caption","runs":[[65535,1024]],"takes":true,"units":2048}: the code answers (1024, Ok(()), Ok(()), Some(Ok(())), 1), the port (2048, Ok(()), Ok(()), Some(Ok(())), 1)
S47203 A2 passed
S47204 KILLED by A1: 1 of 55 vectors are misjudged, the first line 41: {"rule":"caption","runs":[[65536,512]],"takes":true,"units":512}: the code answers (1024, Ok(()), Ok(()), Some(Ok(())), 1), the port (512, Ok(()), Ok(()), Some(Ok(())), 1)
S47204 A2 passed
S47205 KILLED by A1: assertion `left == right` failed on the header's count, the message printing the header line: left: Some(54) right: Some(55)
S47205 A2 passed
S47206 KILLED by A2: assertion `left == right` failed: left: (5, 31, 18, 11, 9) right: (6, 31, 18, 12, 9)
S47206 A1 passed
```

Under S47206, A1 passes as expected: the replacement is a caption of 1,025 letters, which the code
refuses as its vector says, so only the classes A2 holds move.

## The Lean readings

The entry `lean/CaptionBound` was elaborated with its two witnesses before either theorem's proof
was written, and both witnesses built. On the committed tree the formal checker's entry check read
both theorems and both witnesses built, with each of the three covers `STALE` against digests
written as zeros; after the stamp and the rewritten vectors, the same check with `--gate` read
`FORMAL OK`, both theorems clean, both witnesses built, and no `STALE` or `DERIVED_DRIFT` finding.
The ratchet over every entry read `FORMAL OK` with no weakening. `#print axioms` read `propext`,
`Classical.choice` and `Quot.sound` for both theorems, and `propext` and `Quot.sound` for both
witnesses. Scratch examples outside the tree elaborated: 1,022 letters and U+1F600 are taken,
1,023 letters and U+10FFFF are refused, an image's own `Format` refusal stands over 1,024 letters,
and a count of UTF-8 bytes and a check that refuses at the limit each break the first claim; a
false example, 1,025 letters taken, failed to elaborate as its control. No CI job checks the entry:
the test of A1 and A2 and the rows run in CI, and the entry is checked by hand.
