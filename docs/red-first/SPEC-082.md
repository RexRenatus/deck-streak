# Red-first record: SPEC-082

This record is E1's, the first of SPEC-082's three pull requests. Part 1 of E1 delivers A1, A2, A3
and A10 (the wallet's goldens and the constants); the later parts of E1 add A4 to A6, A8, A9, A14
and A19, and E1b and E3 add their criteria's lines when they move them back into the acceptance
fence (SPEC-082 section 3c).

The drafts at da0b1cd compiled and answered nothing: every constant 0, a mint, a cap and a fine of 0
and a clip of `(0, false)`.

The order of work: the SPEC moved out of `docs/specs/planned/` and given its section 3c; the
registry and the goldens; the wallet's tests beside inert drafts that compiled and answered nothing;
and their implementation.

```red-first
A1: red at da0b1cd: assertion `left == right` failed: the mint of {"base_xp":25}; left: Number(0), right: Number(1)
A1: green at fb331f6
A2: red at da0b1cd: assertion `left == right` failed: the cap of {"wallet_at_rollover":4}; left: Number(0), right: Number(1)
A2: green at fb331f6
A10: red at da0b1cd: constants.COIN_MINT_XP_DIVISOR: ours 0, the predecessor's 25
A10: green at fb331f6
```
