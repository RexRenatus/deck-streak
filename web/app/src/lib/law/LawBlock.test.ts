/**
 * @vitest-environment jsdom
 */
import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import LawBlock from './LawBlock.svelte';
import type { LawView } from './law';

// SPEC-077 R10, R12, R13, R14, A20. The law tab shows the law block as the route answers it: the
// lines the block shows, in the server's order; a count the recompute has not stored, or whose port
// is not wired, as pending and never as 0; and SPEC-072's tier distribution beside it.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

function textOf(element: Element | null | undefined): string {
  return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** A synthetic block: shown on its streak and XP, the dues, the leeches and the mastery pending. */
function viewed(overrides: Partial<LawView> = {}): LawView {
  return {
    shown: true,
    levelShown: true,
    lines: ['total_xp', 'streak', 'xp_today'],
    streak: 6,
    xpToday: 35,
    totalXp: 1240,
    level: 7,
    dues: null,
    leeches: null,
    mastery: null,
    ...overrides
  };
}

describe('the law block', () => {
  it('renders a pending count as pending, never zero', () => {
    render(LawBlock, { view: viewed(), tiers: null });

    const pending = screen.queryByRole('list', { name: 'Not counted yet' });
    const items = examined('pending counts', pending ? within(pending).queryAllByRole('listitem') : []);

    // each pending count reads as pending, in the block's order
    expect(items.map(textOf)).toEqual([
      'Law cards due: pending',
      'Law mastery: pending',
      'Active law leeches: pending'
    ]);
    // and nothing in the tab reads it as 0
    const tab = textOf(document.body);
    expect(tab).not.toMatch(/due: 0|mastery: 0|leeches: 0/);
    expect(document.body.querySelectorAll('[data-line="dues"], [data-line="mastery"], [data-line="leeches"]')).toHaveLength(0);
  });
});
