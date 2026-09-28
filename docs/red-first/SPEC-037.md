# Red-first record: SPEC-037

The tests were committed (13f69a9) before the implementation, with `scripts/vendor-packs.py`
stubbed as the old vendoring: it copies every listed file and every new file of a vendored pack, and
applies no exclusion and no scan. Each criterion was run there for its own reason, and again at the
implementation (4acf763). Both shas were re-run from a `git archive` export.

```red-first
A1: red at 13f69a9: AssertionError: Lists differ: ['.packs/scripts/proxy-client-scan.py', '.packs/skills/packs/alpha/examples/demo.md', '.packs/skills/packs/subscription-proxy/SKILL.md', '.packs/skills/packs/subscription-proxy/checks.json', '.packs/skills/packs/subscription-proxy/client/run-headless.sh'] != [] (every excluded fixture file landed in the tree)
A1: green at 4acf763
A2: red at 13f69a9: AssertionError: 0 != 1 : 8 files vendored (the address was copied and the run exited 0)
A2: green at 4acf763
A3: red at 13f69a9: AssertionError: 0 != 1 : 9 files vendored (by --deny-list and by PERSONA_CORE_DENY_LIST alike: the literal was copied and the run exited 0)
A3: green at 4acf763
A4: red at 13f69a9: AssertionError: 0 != 1 : 9 files vendored (the binary was copied and the run exited 0)
A4: green at 4acf763
A5: red at 13f69a9: AssertionError: 0 != 2 : 7 files vendored (the vanished file was skipped and the run exited 0)
A5: green at 4acf763
A6: red at 13f69a9: AssertionError: '0000000000000000000000000000000000000000' != the fixture's commit (the old vendoring never read the commit it copied from)
A6: green at 4acf763
A7: red at 13f69a9: AssertionError: Lists differ: ['carries prose keys: glob', 'carries no globs'] != [] (the manifest's exclusions were prose)
A7: green at 4acf763
```

A1's fixture takes both routes the old vendoring took: files the manifest lists (the scanner, the
proxy pack's rows) and new files of a vendored pack (the reference client, an example). Its client
and scanner name a private literal the run is given, so A1 is green only because an exclusion drops
a file before it is read. The fixture's exclusions are the real manifest's, so A1 holds them against
the names upstream ships today. Two further tests were red at 13f69a9 and green at 4acf763: a second
run changes nothing (`'8 files vendored' != 'vendor-packs: examined 7 file(s), ...'`), and a run
that examines nothing is VOID (`0 != 3 : 1 files vendored`).

At 4acf763, a run against the vendored phoenix-v2 commit e54f39c, into a scratch copy of this tree
and with the maintainer's private list, printed
`vendor-packs: examined 186 file(s), changed 0, new 0, excluded 71, from e54f39c5ad586fc59da33b56bd8e0044af277c25`.
It named the subscription-proxy pack excluded whole, left out 18 upstream packs that are not
vendored, found nothing, and left every one of the copy's 550 files byte-identical.
