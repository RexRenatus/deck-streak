# Red-first record: SPEC-395

The SPEC and ADR-409 were committed first, with the census test and the in-place edit of SPEC-354's
identity table, in one commit pushed alone. Each red below is quoted from CI's run at that red
commit, and the fix commit changes the two units and nothing else.

```red-first
A1: red at ff682db663606b54716d5193b00ea4f96cca997a: AssertionError: Lists differ: ['deploy/systemd/deck-streak-api.service: [343 chars]sed"] != []
A1: green at 3acb4debda69a6393173bb69c2cc879411f082fd
A2: not red: pins the listen settings the base already had, each a loopback address in the census's table; it guards the census's population and proves no new behaviour
A3: red at ff682db663606b54716d5193b00ea4f96cca997a: AssertionError: {'deck-streak-alert@.service': 'link-local', 'deck-strea[609 chars]ket'} != {'deck-streak-api.service': 'any', 'deck-streak-bot.serv[595 chars]mpt'}
A3: green at 3acb4debda69a6393173bb69c2cc879411f082fd
```
