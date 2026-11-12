/**
 * @vitest-environment jsdom
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createApi } from '../api';
import TierAnimation from './TierAnimation.svelte';
import {
  BEATS,
  FEED_PATH,
  celebrations,
  parseFeed,
  reducedMotion,
  type FeedItem,
  type Tier
} from './feed';

// SPEC-084 R10, A16. The Mini App animates each celebration at the tier the router rendered it at:
// a T3 as a two-beat reveal, a T4 as a dice then the message, a T5 as a dice, the message and a
// card that stays until the owner dismisses it. When the device asks for reduced motion, each tier
// shows the same content without motion: the reveal's placeholder, which only moves, is not shown.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The device's answer to the reduced-motion query: `reduce` for that query, and no other. */
function prefer(reduce: boolean): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query === '(prefers-reduced-motion: reduce)',
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined
  }));
}

/** A synthetic item of `tier`. */
function item(tier: Tier): FeedItem {
  return { kind: 'celebration', text: `A synthetic ${tier}`, tier };
}

/** What `tier` shows on a device that does or does not ask for reduced motion. */
function shown(tier: Tier, reduce: boolean) {
  prefer(reduce);
  const { container, unmount } = render(TierAnimation, { props: { item: item(tier) } });
  const beats = [...container.querySelectorAll('[data-beat]')];
  const seen = {
    motion: container.querySelector('[data-tier]')?.getAttribute('data-motion') ?? null,
    beats: beats.map((beat) => beat.getAttribute('data-beat')),
    content: beats
      .filter((beat) => beat.getAttribute('aria-hidden') !== 'true')
      .map((beat) => (beat.textContent ?? '').trim())
  };
  unmount();
  return seen;
}

describe('TierAnimation', () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('animates each tier and shows the same content with reduced motion', () => {
    const expected: Record<string, string[]> = {
      T1: ['message'],
      T2: ['message'],
      T3: ['placeholder', 'message'],
      T4: ['dice', 'message'],
      T5: ['dice', 'message', 'card']
    };
    for (const tier of examined('tiers', ['T1', 'T2', 'T3', 'T4', 'T5'] as const)) {
      const moving = shown(tier, false);
      const still = shown(tier, true);

      // each tier's beats, in order, in motion
      expect(moving.beats, tier).toEqual(expected[tier]);
      expect(moving.motion, tier).toBe('on');
      // the same content without motion: every beat but the placeholder, which only moves
      expect(still.motion, tier).toBe('off');
      expect(still.beats, tier).toEqual(expected[tier].filter((beat) => beat !== 'placeholder'));
      expect(still.content, tier).toEqual(moving.content);
    }
    // the content itself: the dice, the message, and the card's dismissal
    expect(shown('T5', false).content).toEqual(['\u{1f3b0}', 'A synthetic T5', 'Dismiss']);
    expect(shown('T3', false).content).toEqual(['A synthetic T3']);
    // a T0 shows nothing at all, not even an empty item
    const silent = shown('T0', false);
    expect([silent.beats, silent.motion]).toEqual([[], null]);
  });

  it('names the dice and hides the reveal placeholder from a screen reader', () => {
    prefer(false);
    render(TierAnimation, { props: { item: item('T4') } });
    expect(screen.getByRole('img', { name: 'A dice roll' }).textContent).toBe('\u{1f3b0}');
    cleanup();

    render(TierAnimation, { props: { item: item('T3') } });
    const placeholder = document.querySelector('[data-beat="placeholder"]');
    expect([placeholder?.getAttribute('aria-hidden'), placeholder?.textContent]).toEqual([
      'true',
      'Opening…'
    ]);
  });

  it('keeps a T5 card until the owner dismisses it', async () => {
    prefer(false);
    render(TierAnimation, { props: { item: item('T5') } });
    expect(screen.getByText('A synthetic T5')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));

    expect(screen.queryByText('A synthetic T5')).toBeNull();
    expect(document.querySelector('[data-tier]')).toBeNull();
  });

  it('offers no dismissal below T5', () => {
    prefer(false);
    for (const tier of ['T2', 'T3', 'T4'] as const) {
      render(TierAnimation, { props: { item: item(tier) } });
      // the celebration shows, and nothing in it dismisses it
      expect(screen.getByText(`A synthetic ${tier}`).closest('[data-tier]')?.getAttribute('data-tier'), tier).toBe(tier);
      expect(screen.queryByRole('button'), tier).toBeNull();
      cleanup();
    }
  });
});

describe('the ladder feed', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('reads each tier and nothing that is not an item', () => {
    const tiers = examined('tiers', ['T0', 'T1', 'T2', 'T3', 'T4', 'T5'] as const);
    const items = tiers.map((tier) => ({ ...item(tier), created_at: 1 }));
    expect(parseFeed({ items })).toEqual(tiers.map(item));

    // an item missing any field, or of a tier the router never writes, is skipped
    const partial = [
      { text: 'a', tier: 'T2' },
      { kind: 'celebration', tier: 'T2' },
      { kind: 'celebration', text: 'a' },
      { kind: 'celebration', text: 'a', tier: 'T6' },
      { kind: 7, text: 'a', tier: 'T2' },
      { kind: 'celebration', text: 7, tier: 'T2' },
      null
    ];
    expect(parseFeed({ items: [...partial, items[2]] })).toEqual([item('T2')]);
    // a body that is not the feed's
    for (const body of [null, undefined, {}, { items: 'none' }, 'items']) {
      expect(parseFeed(body)).toBeNull();
    }
  });

  it('hands on the served items, or none when the feed serves nothing', async () => {
    const served = [item('T3')];
    expect(await celebrations({ feed: async () => ({ kind: 'ok', value: served }) })).toEqual(served);
    for (const answer of [{ kind: 'unavailable' }, { kind: 'reopen' }] as const) {
      expect(await celebrations({ feed: async () => answer })).toEqual([]);
    }
  });

  it('asks the device about reduced motion by its query', () => {
    prefer(true);
    expect(reducedMotion()).toBe(true);
    prefer(false);
    expect(reducedMotion()).toBe(false);
    vi.stubGlobal('matchMedia', undefined);
    expect(reducedMotion()).toBe(false);
  });

  it('reads the feed from its route on the owner session', async () => {
    const calls: string[] = [];
    const api = createApi({
      launchData: () => 'synthetic-launch',
      fetch: async (input) => {
        calls.push(String(input));
        const body = String(input) === FEED_PATH ? { items: [item('T4')] } : {};
        return new Response(JSON.stringify(body), { status: 200 });
      }
    });
    expect(await api.feed()).toEqual({ kind: 'ok', value: [item('T4')] });
    expect(calls).toEqual(['/api/session', FEED_PATH]);
    expect(FEED_PATH).toBe('/api/notifications/feed');
    expect(BEATS.T0).toEqual([]);
  });
});
