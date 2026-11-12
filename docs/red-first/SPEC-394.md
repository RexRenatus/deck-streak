# Red-first record: SPEC-394

SPEC-394 (R1 to R7, A1 to A4). The SPEC and ADR-408 were committed first (fb3ecb73f9068be73f8b7cf1e205ad131837ceda). The census of A1 to
A4 was committed alone (0167191c0fe25e9835632339b2cf34ae45f333c1), over the base drawing, and each of its four tests read red for its own
criterion. The drawing that draws every crate and every manifest edge was committed next (247e75140cf5bf9915462dc430afd6b553f66fc2), and
the four tests read green on it. Each red below is quoted from the run at its commit.

```red-first
A1: red at 0167191c0fe25e9835632339b2cf34ae45f333c1: AssertionError: findings: 16 (the six crates with no node and ten labels that are not the directory's name)
A2: red at 0167191c0fe25e9835632339b2cf34ae45f333c1: AssertionError: findings: 83 (77 manifest edges no readable arrow draws, three arrows that name a layer, three lines that chain targets)
A3: red at 0167191c0fe25e9835632339b2cf34ae45f333c1: AssertionError: findings: 2 (kernel: outside every layer, xp: outside every layer)
A4: red at 0167191c0fe25e9835632339b2cf34ae45f333c1: AssertionError: findings: 1 (the header names 0 full commits, not one)
A1: green at 247e75140cf5bf9915462dc430afd6b553f66fc2
A2: green at 247e75140cf5bf9915462dc430afd6b553f66fc2
A3: green at 247e75140cf5bf9915462dc430afd6b553f66fc2
A4: green at 247e75140cf5bf9915462dc430afd6b553f66fc2
```
