The owner pins `signers#SHA256:9nZCCavz9v3jGdETXf390MXzqZZ+JMET2dzGJj6ZpRM` as DeckStreak's trust root, in the owner's own words: "go with your recommendation on 14, also send commit for me sign once completed" (owner, 2026-09-30).

# OWNER RULING 2026-09-30: the trust-root bootstrap

This is the first key of `config/owner-allowed-signers`, the file `config/formal.json` names as `owner_signers` (ADR-295).
The key is the owner's own: principal `owner`, type `ssh-ed25519`, and the SHA256 fingerprint named on line 1.

The commit that adds line 1 is signed by that key. That signature is the proof of possession through which the formal
check's ratchet admits the trust-root change line 1 names.

Every later change to the trust root, and every change the ratchet refuses as a weakening, qualifies only through a
ruling in a commit signed by a key this file holds at that change's merge-base.
