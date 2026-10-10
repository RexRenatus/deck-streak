import { describe, expect, it } from 'vitest';
import { OPS, STATUS_WORDS, parseRequest } from './protocol';

// SPEC-350 A6: the review's six operations cross the Worker's wire with exactly their arguments,
// each within the engine's type, and `open` takes an optional list of language tags (R4).
const I64 = 2n ** 63n - 1n;

describe('the study operations on the wire', () => {
  it('each study operation parses its arguments and refuses any other', () => {
    const admitted: Record<string, unknown>[] = [
      { id: 1, op: 'decks' },
      { id: 2, op: 'card' },
      { id: 3, op: 'study', deck: 1n },
      { id: 4, op: 'study', deck: I64 },
      { id: 5, op: 'rate', card: 1001n, rating: 1, ms: 0 },
      { id: 6, op: 'rate', card: I64, rating: 3, ms: 2 ** 32 - 1 },
      { id: 7, op: 'bury', card: 1001n },
      { id: 8, op: 'flag', card: 1001n },
      { id: 9, op: 'open' },
      { id: 10, op: 'open', languages: ['en'] },
      { id: 11, op: 'open', languages: ['zh-CN', 'en'] },
      { id: 12, op: 'open', languages: undefined },
      { id: 13, op: 'open', languages: Array(8).fill('en') },
      { id: 14, op: 'undo-offer' },
      { id: 15, op: 'undo', card: 1001n, step: 0 },
      { id: 16, op: 'undo', card: I64, step: 2 ** 32 - 1 }
    ];
    for (const request of admitted) {
      expect(parseRequest(request), JSON.stringify(request, (_, v) => String(v))).toEqual({ request });
    }

    const refused: [Record<string, unknown>, string][] = [
      [{ id: 1, op: 'decks', deck: 1n }, 'decks takes no deck'],
      [{ id: 1, op: 'card', card: 1n }, 'card takes no card'],
      [{ id: 1, op: 'study' }, "study's deck is malformed"],
      [{ id: 1, op: 'study', deck: 0n }, "study's deck is malformed"],
      [{ id: 1, op: 'study', deck: I64 + 1n }, "study's deck is malformed"],
      [{ id: 1, op: 'study', deck: 1 }, "study's deck is malformed"],
      [{ id: 1, op: 'study', deck: '1' }, "study's deck is malformed"],
      [{ id: 1, op: 'study', deck: 1n, card: 1n }, 'study takes no card'],
      [{ id: 1, op: 'rate', rating: 3, ms: 0 }, "rate's card is malformed"],
      [{ id: 1, op: 'rate', card: 0n, rating: 3, ms: 0 }, "rate's card is malformed"],
      [{ id: 1, op: 'rate', card: 1001, rating: 3, ms: 0 }, "rate's card is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 0, ms: 0 }, "rate's rating is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 5, ms: 0 }, "rate's rating is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 2.5, ms: 0 }, "rate's rating is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 2, ms: 0 }, "rate's rating is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 4, ms: 0 }, "rate's rating is malformed"],
      [{ id: 1, op: 'answer', rating: 2, ms: 0 }, 'unknown operation answer'],
      [{ id: 1, op: 'answer', rating: 3, ms: 0 }, 'unknown operation answer'],
      [{ id: 1, op: 'rate', card: 1001n, rating: 3 }, "rate's ms is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 3, ms: -1 }, "rate's ms is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 3, ms: 1.5 }, "rate's ms is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 3, ms: 2 ** 32 }, "rate's ms is malformed"],
      [{ id: 1, op: 'rate', card: 1001n, rating: 3, ms: 0, deck: 1n }, 'rate takes no deck'],
      [{ id: 1, op: 'bury' }, "bury's card is malformed"],
      [{ id: 1, op: 'bury', card: -1n }, "bury's card is malformed"],
      [{ id: 1, op: 'bury', card: 1001n, rating: 3 }, 'bury takes no rating'],
      [{ id: 1, op: 'flag' }, "flag's card is malformed"],
      [{ id: 1, op: 'flag', card: 1001n, flag: 1 }, 'flag takes no flag'],
      [{ id: 1, op: 'open', languages: [] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: 'en' }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: [1] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: ['en', ''] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: ['en_US'] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: ['english'] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: ['zh-CN-x'] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: Array(9).fill('en') }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: [['en']] }, "open's languages is malformed"],
      [{ id: 1, op: 'open', languages: ['en'], sql: 'x' }, 'open takes no sql'],
      [{ id: 1, op: 'undo' }, "undo's card is malformed"],
      [{ id: 1, op: 'undo', card: 0n, step: 1 }, "undo's card is malformed"],
      [{ id: 1, op: 'undo', card: 1001, step: 1 }, "undo's card is malformed"],
      [{ id: 1, op: 'undo', card: 1001n }, "undo's step is malformed"],
      [{ id: 1, op: 'undo', card: 1001n, step: -1 }, "undo's step is malformed"],
      [{ id: 1, op: 'undo', card: 1001n, step: 1.5 }, "undo's step is malformed"],
      [{ id: 1, op: 'undo', card: 1001n, step: 2 ** 32 }, "undo's step is malformed"],
      [{ id: 1, op: 'undo', card: 1001n, step: 1, rating: 3 }, 'undo takes no rating'],
      [{ id: 1, op: 'undo-offer', card: 1001n }, 'undo-offer takes no card']
    ];
    for (const [request, message] of refused) {
      expect(parseRequest(request), message).toEqual({ id: 1, message });
    }

    // the six join the operations the Worker serves, after DEV's seven: the queue-head answer is
    // gone, and a grade is recorded only by rate (SPEC-365 R9); the undo's offer follows the undo
    // (SPEC-371 R12)
    expect(OPS).toEqual([
      'open',
      'seed',
      'next',
      'undo',
      'undo-offer',
      'snapshot',
      'memory',
      'close',
      'decks',
      'study',
      'card',
      'rate',
      'bury',
      'flag',
      'faces',
      // the sync's two join before the credential's two (SPEC-364 R17, R18)
      'sync-login',
      'sync',
      'credential-status',
      'credential-forget',
      // the full sync's choice joins after the credential's two (SPEC-377 R6)
      'choice-count',
      'choice-confirm',
      'choice-cancel',
      'unsynced'
    ]);
  });

  it('the faces operation names the card it asks for', () => {
    // SPEC-350 A24, ADR-361 D12: faces carries the shown card's id, within the engine's type, and
    // nothing else, so the page names no media file and no limit
    expect(parseRequest({ id: 9, op: 'faces', card: 1001n })).toEqual({
      request: { id: 9, op: 'faces', card: 1001n }
    });
    expect(parseRequest({ id: 9, op: 'faces', card: I64 })).toEqual({ request: { id: 9, op: 'faces', card: I64 } });
    expect(parseRequest({ id: 9, op: 'faces' })).toEqual({ id: 9, message: "faces's card is malformed" });
    expect(parseRequest({ id: 9, op: 'faces', card: 0n })).toEqual({ id: 9, message: "faces's card is malformed" });
    expect(parseRequest({ id: 9, op: 'faces', card: 1001n, names: ['cat.mp3'] })).toEqual({
      id: 9,
      message: 'faces takes no names'
    });
  });

  it('the credential ops answer a status word, and take no argument', () => {
    // SPEC-363 B11 (R15): the two credential operations carry nothing but their id, so no key, user
    // or password crosses the wire to the Worker through them, and each answers one of five words
    expect(parseRequest({ id: 1, op: 'credential-status' })).toEqual({ request: { id: 1, op: 'credential-status' } });
    expect(parseRequest({ id: 2, op: 'credential-forget' })).toEqual({ request: { id: 2, op: 'credential-forget' } });
    expect(parseRequest({ id: 3, op: 'credential-status', key: 'k' })).toEqual({
      id: 3,
      message: 'credential-status takes no key'
    });
    expect(parseRequest({ id: 4, op: 'credential-forget', user: 'u' })).toEqual({
      id: 4,
      message: 'credential-forget takes no user'
    });
    expect(STATUS_WORDS).toEqual(['absent', 'sealed', 'held', 'needs-sign-in', 'offline']);
  });

  it('the sync login takes a bounded user and password, and a sync takes no argument', () => {
    // SPEC-364 R17, R18: each of the login's two strings is non-empty and at most 1024 characters
    const longest = 'x'.repeat(1024);
    const admitted: Record<string, unknown>[] = [
      { id: 1, op: 'sync-login', user: 'u', password: 'p' },
      { id: 2, op: 'sync-login', user: longest, password: longest },
      { id: 3, op: 'sync' }
    ];
    for (const request of admitted) {
      expect(parseRequest(request), String(request.id)).toEqual({ request });
    }
    const refused: [Record<string, unknown>, string][] = [
      [{ id: 1, op: 'sync-login', password: 'p' }, "sync-login's user is malformed"],
      [{ id: 1, op: 'sync-login', user: '', password: 'p' }, "sync-login's user is malformed"],
      [{ id: 1, op: 'sync-login', user: `${longest}x`, password: 'p' }, "sync-login's user is malformed"],
      [{ id: 1, op: 'sync-login', user: ['u'], password: 'p' }, "sync-login's user is malformed"],
      [{ id: 1, op: 'sync-login', user: 'u' }, "sync-login's password is malformed"],
      [{ id: 1, op: 'sync-login', user: 'u', password: '' }, "sync-login's password is malformed"],
      [{ id: 1, op: 'sync-login', user: 'u', password: `${longest}x` }, "sync-login's password is malformed"],
      [{ id: 1, op: 'sync-login', user: 'u', password: 1 }, "sync-login's password is malformed"],
      [{ id: 1, op: 'sync-login', user: 'u', password: 'p', key: 'k' }, 'sync-login takes no key'],
      [{ id: 1, op: 'sync', key: 'k' }, 'sync takes no key']
    ];
    for (const [request, message] of refused) {
      expect(parseRequest(request), message).toEqual({ id: 1, message });
    }
  });
});

describe('the choice operations on the wire', () => {
  it('the choice operations carry no path, no id set and no snapshot answer', () => {
    // SPEC-377 R6, A8: the page names a direction and nothing else; the Worker reads the snapshot
    // answer itself and the web engine names every file, so anything else is refused before any
    // engine call
    const admitted: Record<string, unknown>[] = [
      { id: 1, op: 'choice-count' },
      { id: 2, op: 'choice-confirm', direction: 'upload' },
      { id: 3, op: 'choice-confirm', direction: 'download' },
      { id: 4, op: 'choice-cancel' },
      { id: 5, op: 'unsynced' }
    ];
    for (const request of admitted) {
      expect(parseRequest(request), String(request.id)).toEqual({ request });
    }
    const refused: [Record<string, unknown>, string][] = [
      [{ id: 1, op: 'choice-count', path: '/deck-streak/server-1.anki2' }, 'choice-count takes no path'],
      [{ id: 1, op: 'choice-count', required: 2 }, 'choice-count takes no required'],
      [{ id: 1, op: 'choice-count', key: 'k' }, 'choice-count takes no key'],
      [{ id: 1, op: 'choice-confirm' }, "choice-confirm's direction is malformed"],
      [{ id: 1, op: 'choice-confirm', direction: 'both' }, "choice-confirm's direction is malformed"],
      [{ id: 1, op: 'choice-confirm', direction: 0 }, "choice-confirm's direction is malformed"],
      [{ id: 1, op: 'choice-confirm', direction: ['upload'] }, "choice-confirm's direction is malformed"],
      [{ id: 1, op: 'choice-confirm', direction: 'Upload' }, "choice-confirm's direction is malformed"],
      [{ id: 1, op: 'choice-confirm', direction: 'upload', found: true }, 'choice-confirm takes no found'],
      [{ id: 1, op: 'choice-confirm', direction: 'upload', snapshot: { found: true } }, 'choice-confirm takes no snapshot'],
      [{ id: 1, op: 'choice-confirm', direction: 'download', path: '/deck-streak/backup-1.anki2' }, 'choice-confirm takes no path'],
      [{ id: 1, op: 'choice-confirm', direction: 'download', ids: [1] }, 'choice-confirm takes no ids'],
      [{ id: 1, op: 'choice-confirm', direction: 'download', key: 'k' }, 'choice-confirm takes no key'],
      [{ id: 1, op: 'choice-cancel', path: '/deck-streak/collection.anki2' }, 'choice-cancel takes no path'],
      [{ id: 1, op: 'unsynced', reviews: 3 }, 'unsynced takes no reviews'],
      [{ id: 1, op: 'choice-write' }, 'unknown operation choice-write']
    ];
    for (const [request, message] of refused) {
      expect(parseRequest(request), message).toEqual({ id: 1, message });
    }
    console.log(`examined ${admitted.length} admitted and ${refused.length} refused choice requests`);
    expect((OPS as readonly string[]).filter((op) => op.startsWith('choice-') || op === 'unsynced')).toEqual([
      'choice-count',
      'choice-confirm',
      'choice-cancel',
      'unsynced'
    ]);
  });
});
