# Red-first record: SPEC-474

SPEC-474 (R1 to R9, A1 to A3). The red commit holds the SPEC, ADR-491 and the schematic, the web
test, the writer and its arm in `formal/lean/Formal/Vectors.lean`, and the Lean entry with its port
lacking the opener rule (`docs-mermaid-read.js:295-301`), with the vectors that port writes. No line
of the reader changes in this delivery, so each red is its criterion's assertion over the reader as
it ships, never a missing name: at the red commit the entry builds and proves the claim false of its
own port, the web test reads every vector, and both of its tests fail by assertion.

Every red and green below was read locally, each criterion by its own fence line from the
repository root. The repository's CI has no formal job, so A1's checker card was read by the
builder at the green commit. The delivery adds no Rust file, so the gate's red-first stage reads it
outside its domain, and this record carries the reds.

```red-first
A1: red at 40c8d0d4dd954f6adcd4c7dce4c4ca9a4ca9e31b: Formal.DocsMermaidRead.the_reader_without_its_opener_rule_leaves_a_mermaid_line_unnamed : ¬NamedReading readMermaid
A1: green at 7acab2713d0ffd78abfd3ce7562142d2f06ff0e9
A2: red at 40c8d0d4dd954f6adcd4c7dce4c4ca9a4ca9e31b: 623 of 841 vectors read otherwise: expected 623 to be +0 // Object.is equality
A2: green at 7acab2713d0ffd78abfd3ce7562142d2f06ff0e9
A3: red at 40c8d0d4dd954f6adcd4c7dce4c4ca9a4ca9e31b: AssertionError: expected [] to include 'witness.opener-in-a-tilde-fence'
A3: green at 7acab2713d0ffd78abfd3ce7562142d2f06ff0e9
```

- **A1.** The red is the `#check` of the entry's one theorem at the red commit, read from a file
  outside the tree that imports the module: `lake build Formal.DocsMermaidRead` builds there, and
  the theorem states the claim false of the port, which lacks the opener rule. The green is the same
  build at the green commit, where the theorem states the claim of the full port and the two
  witnesses state it false of the port without its opener rule and of the port without its info
  rule, each resting only on the names `config/formal.json` allows; and the formal checker's card
  for `lean/DocsMermaidRead` reads the theorem clean, both witnesses caught, and no finding.
- **A2.** At the red commit the reader refuses, by the opener rule's name, lines that the vectors of
  the port without that rule do not, on 623 of the 841 vectors; the quote is the end of the test's
  first assertion error. At the green commit the vectors hold the opener rule's refusals, and the
  reader answers all 841. Between the two commits the writer's output changed on exactly those 623
  lines, each only by added refusals of the opener rule's name.
- **A3.** At the red commit the reader without its opener rule agrees with every vector of the port
  without that rule, so the planted fault is not caught. At the green commit it reads otherwise at
  the witness's text, and the literal case reads as R7 states.
- Each fence line selected its one test: one failed and one skipped at the red commit, one passed
  and one skipped at the green commit. The whole web suite ran 685 tests at the red commit, of which
  683 passed and exactly these two failed, and all 685 passed at the green commit.
