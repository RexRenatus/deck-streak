# Red-first record: SPEC-044

SPEC-044's criteria over pack rows were re-planned before the build (ac1a201), because no public test
can run a pack's row (ADR-069). The SPEC, ADR-044 and the schematic were then promoted (47ba8fa), and
the public templates, the golden readings, the example roster and the private globs committed
(3ccf3f7). The engine was built in three stages, each committed red against stubs that compile and
fail by assertion, then green: the templates (A1, A3), the roster and instantiation (A2, A7), and the
reader, the frontmatter and the band (A4, A5, A6, A8). Each red was observed with
`cargo test -p deck-streak-agent` on the tree its red commit holds. Where a failure named a path of
the machine that ran it, its line gives the path from the repository's root.

```red-first
A1: red at 5e89e7e: assertion `left == right` failed: the engine carries every template of agent/personas, and no other; left: 0, right: 18 (examined 18; the stub compiled in no template)
A1: green at fd126bc
A2: red at 3e4fb3a: the roster fills the name: Err(TopicUnbound) (the stub roster bound no topic and filled no slot)
A2: green at 742ef31
A3: red at 5e89e7e: assertion failed: matches!(Template::parse(&bio), Err(PersonaError::SlotFilled { slot: "bio" })) (the stub refused the filled copy only as unreadable, never as a filled slot)
A3: green at fd126bc
A4: red at ca1b92a: assertion `left == right` failed; left: [], right: ["leeches@law/evidence", "lapses@law/evidence"] (the stub frontmatter declared none of the reads made)
A4: green at 43c7503
A5: red at ca1b92a: another subject's read is refused: Ok(Some(Recall { source: Leeches, entries: 1 })) (the stub reader asked the port for another subject)
A5: green at 43c7503
A6: red at ca1b92a: agent/golden/daily-reading/law/law-evidence.output.md opens with the engine's frontmatter (the stub frontmatter declared no read where the golden declares two)
A6: green at 43c7503
A7: red at 3e4fb3a: a slot that is not one of the four is refused (the stub roster read a fifth slot)
A7: green at 742ef31
A8: red at ca1b92a: assertion `left == right` failed: the live band, when its port is wired and answers; left: Some(A2), right: Some(B1)
A8: green at 43c7503
```

The five rows of `scripts/mutation-rows.d/S04400-S04499.json` (the subject filter, the journal
exclusion, the band's source, the one-line slot and the band on a language output only) were proved
KILLED on d5a1362, each target restored byte for byte.

B1 and B2 (SPEC-044 §3a) have no line here: the box run judges them over the committed tree, and
its verdict is the pull request's `box/packs` status (ADR-069).
