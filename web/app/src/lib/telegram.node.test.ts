import { describe, expect, it } from 'vitest';

// SPEC-057 A24. In Node there is no `window`: the app's wrapper is built over nothing, and says
// so. The module is imported inside the test, since building the wrapper is what loads it.
describe('the app wrapper with no page', () => {
  it('is outside Telegram when there is no window', async () => {
    expect(typeof window).toBe('undefined');

    const { telegram } = await import('./telegram.svelte');

    expect([telegram.inside, telegram.platform, telegram.startParam]).toEqual([false, 'unknown', null]);
  });
});
