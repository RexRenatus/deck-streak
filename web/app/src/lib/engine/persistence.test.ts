import { describe, expect, it } from 'vitest';
import { requestPersistence, type PersistentStorage } from './persistence';

// SPEC-338 A12: the page asks for persistent storage and shows the browser's answer as it is.
describe('requestPersistence', () => {
  it('the persistence answer is persisted, not-persisted or unsupported', async () => {
    const asked: string[] = [];
    const storage = (persisted: boolean | Error, persist: boolean | Error): PersistentStorage => ({
      persisted: async () => {
        asked.push('persisted');
        if (persisted instanceof Error) throw persisted;
        return persisted;
      },
      persist: async () => {
        asked.push('persist');
        if (persist instanceof Error) throw persist;
        return persist;
      }
    });
    const cases: [string, PersistentStorage | undefined, string, string[]][] = [
      ['already persisted', storage(true, false), 'persisted', ['persisted']],
      ['granted on request', storage(false, true), 'persisted', ['persisted', 'persist']],
      ['declined on request', storage(false, false), 'not-persisted', ['persisted', 'persist']],
      ['the request throws', storage(false, new Error('denied')), 'not-persisted', ['persisted', 'persist']],
      ['the query throws', storage(new Error('denied'), true), 'not-persisted', ['persisted']],
      ['no persist()', { persisted: async () => false }, 'unsupported', []],
      ['no persisted()', { persist: async () => true }, 'unsupported', []],
      ['no storage manager', undefined, 'unsupported', []]
    ];
    expect(cases.length).toBeGreaterThan(0);
    for (const [name, given, expected, calls] of cases) {
      asked.length = 0;
      expect([name, await requestPersistence(given)]).toEqual([name, expected]);
      expect([name, asked]).toEqual([name, calls]);
    }
  });

  it('with no storage given, the request reads the browser navigator', async () => {
    const before = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { storage: { persisted: async () => false, persist: async () => true } }
    });
    try {
      expect(await requestPersistence()).toBe('persisted');
      Object.defineProperty(globalThis, 'navigator', { configurable: true, value: undefined });
      expect(await requestPersistence()).toBe('unsupported');
    } finally {
      if (before) Object.defineProperty(globalThis, 'navigator', before);
    }
  });
});
