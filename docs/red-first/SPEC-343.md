# Red-first record: SPEC-343

The SPEC, its schematic, ADR-354 and the context map's line were committed first, and the
manifests, the lockfile, the crate's skeleton and the tests' fakes next. Each criterion's test was
then committed before the code that turns it green, and each red below is quoted from the run at
the red commit.

```red-first
A10: red at 06e4914d: assertion `left == right` failed: left: ["http://push.synthetic.invalid", "http://push.synthetic.invalid:8080", "http://localhost.synthetic.invalid", ...] (all 13 refused origins admitted), right: []
A16: red at 06e4914d: assertion `left == right` failed: left: None, right: Some(OffTheList)
A10: green at e90b953f
A1: red at beb86050: assertion `left == right` failed: left: Failed(Request), right: Delivered
A2: red at beb86050: assertion `left == right` failed: left: Failed(Request), right: Delivered
A20: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
A21: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
A22: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
```
