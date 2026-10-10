import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-385 R13, A19. Every string the link page, the sign-in page and the sign-in methods screen
// show is a message key in each of the seven locale files, present and not empty; English carries
// §2b's wording, and every locale carries the date placeholder English carries.
const MESSAGES = join(resolve(import.meta.dirname, '../..'), 'messages');

/** SPEC-385 §2b: each key, with its English. */
const ENGLISH: Readonly<Record<string, string>> = {
  signin_title: 'Sign in',
  signin_with_passkey: 'Sign in with a passkey',
  signin_no_webauthn:
    "This browser can't use passkeys. Open DeckStreak in a browser that can, or reopen it from Telegram.",
  signin_not_linked: 'No passkey is linked yet. Link one from DeckStreak in Telegram.',
  passkey_refused: "That passkey can't sign in to DeckStreak.",
  passkey_start_again: 'That attempt expired or was already used. Start again.',
  passkey_too_many: 'Too many attempts. Wait a minute, then start again.',
  passkey_off: "Passkeys aren't available here.",
  passkey_cancelled: 'No passkey was used. You can start again.',
  link_title: 'Link a passkey',
  link_create: 'Create a passkey',
  link_open_in_browser: 'Open this link in your browser to create a passkey.',
  link_no_code: 'This link has no code. Get a new one from DeckStreak in Telegram.',
  link_code_spent:
    'This link expired or was already used. Get a new one from DeckStreak in Telegram.',
  link_done: 'Passkey created. You can now sign in to DeckStreak in this browser.',
  link_already: 'This passkey is already linked.',
  methods_title: 'Sign-in methods',
  methods_telegram: 'Telegram',
  methods_passkey: 'Passkey',
  methods_added: 'Added {date}',
  methods_link_passkey: 'Link a passkey',
  methods_remove: 'Remove',
  methods_remove_confirm: 'Remove this passkey? You can still sign in with Telegram.',
  methods_remove_keep: 'Keep',
  methods_reopen: 'Reopen DeckStreak from Telegram, then try again.'
};

/** The project's seven locales. */
const LOCALES = ['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant'] as const;

describe('the passkey wording', () => {
  it('every locale carries the passkey keys', () => {
    const keys = Object.keys(ENGLISH);
    const total = LOCALES.length * keys.length;
    let judged = 0;
    for (const locale of LOCALES) {
      const messages = JSON.parse(readFileSync(join(MESSAGES, `${locale}.json`), 'utf8')) as Record<
        string,
        unknown
      >;
      for (const key of keys) {
        const text = messages[key];
        expect([locale, key, typeof text === 'string' && text.trim() !== '']).toEqual([
          locale,
          key,
          true
        ]);
        if (locale === 'en') expect([key, text]).toEqual([key, ENGLISH[key]]);
        if (key === 'methods_added') expect([locale, String(text).includes('{date}')]).toEqual([locale, true]);
        judged += 1;
      }
    }
    console.log(`examined ${judged} of ${total} locale keys`);
    expect([keys.length, judged]).toEqual([25, total]);
    // the fixed list is every locale the project has a file for
    const files = readdirSync(MESSAGES)
      .filter((name) => name.endsWith('.json'))
      .map((name) => name.slice(0, -'.json'.length))
      .sort();
    expect(files).toEqual([...LOCALES].sort());
  });
});
