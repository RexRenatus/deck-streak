# Red-first record: SPEC-323

The SPEC, its schematic, ADR-323 and the SPEC-026 and ADR-026 amendments were committed first
(ce0e8705). The two tests were then committed alone at 36cd061d, with the loader's `TOP_KEYS`,
field and `parse` change, `check()` without the refusal, and the policy file carrying
`"replies": []`, and each ran by its exact name before the implementation. Both fail by assertion.
Each run reads `running 10 tests` for the target and `test result: FAILED. 8 passed; 2 failed`.

```red-first
A1: red at 36cd061d: assertion `left == right` failed: every committed golden is a notification or a declared reply; left: 26 faults, the first "crates/bot/tests/messages/badges-failed.msg.json: duty bot-commands is not a declared reply", right: []
A2: red at 36cd061d: a reply that names a declared kind is refused (the entry `alert` parsed, not Malformed)
A1: green at 552232bf
A2: green at 552232bf
```

Mutation rows, proved at 69f3bfbb (`rows: examined 5: killed 5, survived 0, void 0`):

- S32300: KILLED
- S32301: KILLED
- S32302: KILLED
- S32303: KILLED
- S32304: KILLED
