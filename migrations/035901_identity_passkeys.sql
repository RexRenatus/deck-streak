-- SPEC-359 R5, R6, R11: the owner's passkeys, owned by identity (docs/CONTEXT-MAP.md).
--
-- One row per registered credential, always the configured owner's: no route creates an account.
-- `credential_id` is unique, and that key IS the refusal of a credential registered twice
-- (`already_linked`). `user_handle` is the random 16-byte handle every passkey of the owner shares,
-- never personal data. `credential` is the library's serialized credential, which a sign-in
-- updates beside `counter` and `backup_state`; `counter` changes only by a compare-and-swap on the
-- value the counter rule read (ADR-370 D3). Instants are epoch milliseconds, and `last_used_at` is
-- NULL until the first sign-in. The table is exported and erased.
CREATE TABLE passkeys (
    id INTEGER PRIMARY KEY,
    telegram_user_id INTEGER NOT NULL CHECK (telegram_user_id > 0),
    credential_id BLOB NOT NULL UNIQUE CHECK (length(credential_id) > 0),
    user_handle BLOB NOT NULL CHECK (length(user_handle) = 16),
    credential TEXT NOT NULL CHECK (length(credential) > 0),
    counter INTEGER NOT NULL CHECK (counter >= 0 AND counter <= 4294967295),
    backup_state INTEGER NOT NULL CHECK (backup_state IN (0, 1)),
    created_at INTEGER NOT NULL,
    last_used_at INTEGER
) STRICT;
