# Red-first record: SPEC-617

SPEC-617 (R1 to R5, A1 and A2). The SPEC, ADR-639, the schematic and the changelog fragment were
committed first. The two criteria's tests were committed next with the resolver unchanged: the
census takes the root manifest's table but finds no rename in it, so each test read red for its
own criterion by assertion. The resolver changed alone after them, and both tests read green. Each
red below is quoted from the run at its commit.
