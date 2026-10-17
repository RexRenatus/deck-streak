/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import BadgeGallery from './BadgeGallery.svelte';
import type { BadgesView } from './badges';

// SPEC-073 R16, R19, A23. The gallery shows the earned badges as the server orders them, newest
// first, and every locked badge with its criteria; a locked study badge carries its input against
// its threshold, and a badge whose input the server does not keep shows no progress at all.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A synthetic view: two earned badges, newest first, and three locked ones of three families. */
function viewed(overrides: Partial<BadgesView> = {}): BadgesView {
  return {
    earned: [
      { key: 'centurion_day', tier: 0, name: 'Centurion Day', emoji: '💯', studyDay: '2025-01-13' },
      { key: 'week_warrior', tier: 0, name: 'Week Warrior', emoji: '🔥', studyDay: '2025-01-12' }
    ],
    locked: [
      {
        key: 'monthly_monk',
        name: 'Monthly Monk',
        emoji: '🧘',
        criteria: '30-day streak',
        family: 'study',
        progress: { value: 3, threshold: 30 }
      },
      {
        key: 'first_page',
        name: 'First Page',
        emoji: '📖',
        criteria: 'Logged your first reading session',
        family: 'habit',
        progress: null
      },
      {
        key: 'focus_initiate',
        name: 'Focus Initiate',
        emoji: '⏳',
        criteria: 'Completed your first focus block',
        family: 'focus',
        progress: null
      }
    ],
    ...overrides
  };
}

/** The items of the list the gallery names `name`, or none when there is no such list. */
function itemsOf(name: string): HTMLElement[] {
  const list = screen.queryByRole('list', { name });
  return list === null ? [] : Array.from(list.querySelectorAll<HTMLElement>(':scope > li'));
}

/** An element's text with its whitespace collapsed, or the empty string for no element. */
function textOf(element: Element | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** The locked badge whose key is `key`. */
function lockedBadge(key: string): HTMLElement | undefined {
  return itemsOf('Locked badges').find((item) => item.dataset.key === key);
}

describe('BadgeGallery', () => {
  it('shows a locked badge with its criteria and progress', () => {
    render(BadgeGallery, { props: { view: viewed() } });

    const monk = lockedBadge('monthly_monk');
    expect(textOf(monk)).toBe('🧘 Monthly Monk 30-day streak 3 of 30');
    const bar = monk?.querySelector('meter') ?? null;
    expect([bar?.value, bar?.max, bar?.getAttribute('aria-label')]).toEqual([
      3,
      30,
      'Monthly Monk progress'
    ]);
  });

  it('shows a badge whose input is not kept with its criteria and no progress', () => {
    render(BadgeGallery, { props: { view: viewed() } });

    const shown = ['first_page', 'focus_initiate'].map((key) => {
      const badge = lockedBadge(key);
      const bar = badge === undefined ? 'missing' : badge.querySelector('meter');
      return [textOf(badge), badge?.dataset.family, bar];
    });
    expect(shown).toEqual([
      ['📖 First Page Logged your first reading session', 'habit', null],
      ['⏳ Focus Initiate Completed your first focus block', 'focus', null]
    ]);
  });

  it('lists the earned badges newest first, each with the day it was earned', () => {
    render(BadgeGallery, { props: { view: viewed() } });

    const earned = itemsOf('Earned badges').map(textOf);
    expect(earned).toEqual([
      '💯 Centurion Day Earned 2025-01-13',
      '🔥 Week Warrior Earned 2025-01-12'
    ]);
    examined('earned badge(s)', earned);
  });

  it('lists every locked badge the server sends, in its order', () => {
    render(BadgeGallery, { props: { view: viewed() } });

    const keys = itemsOf('Locked badges').map((item) => item.dataset.key);
    expect(keys).toEqual(['monthly_monk', 'first_page', 'focus_initiate']);
    examined('locked badge(s)', keys);
  });

  it('says no badge is earned yet, and lists none, when the owner has none', () => {
    render(BadgeGallery, { props: { view: viewed({ earned: [] }) } });

    expect(textOf(screen.queryByText(/No badges yet/) ?? undefined)).toBe(
      'No badges yet — study to earn your first!'
    );
    expect(screen.queryByRole('list', { name: 'Earned badges' })).toBeNull();
  });

  it('names its two sections', () => {
    render(BadgeGallery, { props: { view: viewed() } });

    const headings = screen.queryAllByRole('heading').map((heading) => textOf(heading));
    expect(headings).toEqual(['Earned badges', 'Locked badges']);
  });
});
