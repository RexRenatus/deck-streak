# Sealed sync: the privacy text the cut-over installs

This is the text the privacy page gains when DeckStreak's own sync replaces the sync path that
serves the stock Anki apps. Until that change, `PRIVACY.md` describes the sync that runs today, and
nothing on this page is in effect. It belongs to SPEC-583 and ADR-602.

## What the server can never read

Every sync op is sealed on your device before it leaves it. The sealed part holds your notes and
their fields, your media, your deck names and note type names, your reviews, your palace data and
your tags. DeckStreak's server stores each of them as ciphertext and holds no key that opens them:
the key that seals them is made on your device and reaches the server only wrapped for your own
devices, your own passkeys and your printed recovery key. Signing in with an email code opens your
account; it never opens your data.

In the web app, the code that seals and opens your data is the app DeckStreak's site serves you,
so on the web the seal holds against the server's store, its backups and its logs, and you trust
the released app the site serves.

## What the server holds

- **Op heads.** For each op: its format version, the key epoch it was sealed under, the random id
  of the device that made it, that device's op number, its clock time, whether it adds, replaces or
  removes, a keyed hash of what it replaces that reveals no card, note or field, the revision it
  was based on, and its random nonce. The server orders and merges ops by these alone.
- **Sizes.** Each op's sealed size, rounded up to a multiple of 256 bytes, and each media object's
  sealed size, rounded up by a padding rule, which a per-learner storage quota counts.
- **Wrapped keys.** For each of your devices, passkeys and your recovery key: a public key, and the
  key that seals your data, wrapped so that only that device, passkey or printed key can unwrap it.
  The list of your devices is itself sealed.
- **One daily record per study day.** The date, the number of reviews, the minutes and the
  XP granted, which your device sends so the server can keep your streaks, freezes, coins, quests
  and reminder budgets. The server checks each record against what it can count and refuses one that
  does not fit.

Each is kept until you erase it. Its lawful basis is the contract: the sync and the game you use
need it.

## Your copy, erasing it, and losing your devices

- An export from the service gives you the daily records and your sealed ops as ciphertext; your
  device opens them and exports your notes, reviews and the rest in the clear.
- An erase deletes your sealed ops, your media objects, your wrapped keys, your device list and
  your daily records. With every wrapped key gone, no sealed copy can be opened again.
- If you lose a device, another device, a passkey or your printed recovery key unlocks your data
  on a new one. If you lose all of them, your data cannot be recovered, by you or by DeckStreak.
