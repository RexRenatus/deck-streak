/**
 * @vitest-environment jsdom
 */
import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import LawBlock from './LawBlock.svelte';
import type { LawTiersView, LawView } from './law';

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

// SPEC-408 R1, R4, A1. The law block's mastery line says what law mastery measures, in plain words
// beside its figure, and no other line does.
const LAW_ABOUT =
  'Law mastery starts at 100% and drops 3 points for each active law leech, a law card you keep forgetting, never below 70%; it does not measure how much law you know.';
const COURSE_ABOUT =
  'Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0.';

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

  it('shows the lines the block shows, in the server order, with the level when it names it', () => {
    const view = viewed({
      lines: ['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches'],
      dues: 4,
      leeches: 2,
      mastery: 93.6
    });
    render(LawBlock, { view, tiers: TIERS });

    const block = screen.queryByRole('list', { name: 'Law' });
    const lines = examined('shown lines', block ? within(block).queryAllByRole('listitem') : []);
    expect(lines.map((line) => line.dataset.line)).toEqual(['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches']);
    expect(lines.map(textOf)).toEqual([
      'Lifetime law XP: 1240, level 7',
      'Law streak: 6 days',
      'Law XP today: 35',
      'Law cards due: 4',
      'Law mastery: 94% Law mastery starts at 100% and drops 3 points for each active law leech, a law card you keep forgetting, never below 70%; it does not measure how much law you know.',
      'Active law leeches: 2'
    ]);
    // nothing is pending, so there is no list of pending counts
    expect(screen.queryByRole('list', { name: 'Not counted yet' })).toBeNull();
    expect(screen.queryByText('Not counted yet')).toBeNull();
    expect(screen.queryByRole('region', { name: 'Law' })).not.toBeNull();
  });

  it('the mastery line says what law mastery measures and no other line does', () => {
    const view = viewed({
      lines: ['total_xp', 'streak', 'xp_today', 'dues', 'mastery', 'leeches'],
      dues: 4,
      leeches: 2,
      mastery: 93.6
    });
    const { container } = render(LawBlock, { view, tiers: TIERS });

    const mastery = container.querySelector('li[data-line="mastery"]');
    expect(textOf(mastery?.querySelector('[data-mastery-about="law"]'))).toBe(LAW_ABOUT);
    const shown = examined('shown mastery descriptions', [...container.querySelectorAll('[data-mastery-about]')]);
    expect(shown).toHaveLength(1);
    expect(textOf(mastery)).toBe(`Law mastery: 94% ${LAW_ABOUT}`);
    expect(textOf(container)).not.toContain(COURSE_ABOUT);

    // a block whose mastery is still pending holds no sentence, and its pending line is unchanged
    document.body.innerHTML = '';
    const pending = render(LawBlock, { view: viewed(), tiers: null });
    expect(pending.container.querySelectorAll('[data-mastery-about]')).toHaveLength(0);
    expect(textOf(pending.container.querySelector('[data-pending="mastery"]'))).toBe('Law mastery: pending');
  });

  it('names no level when the server does not, and keeps the server order', () => {
    render(LawBlock, { view: viewed({ levelShown: false, lines: ['xp_today', 'total_xp'] }), tiers: TIERS });

    const block = screen.queryByRole('list', { name: 'Law' });
    const lines = examined('shown lines', block ? within(block).queryAllByRole('listitem') : []);
    expect(lines.map(textOf)).toEqual(['Law XP today: 35', 'Lifetime law XP: 1240']);
  });

  it('says there is no law activity when the block is omitted, and still lists each pending count', () => {
    render(LawBlock, { view: viewed({ shown: false, levelShown: false, lines: [], streak: 0, xpToday: 0, totalXp: 0, level: 1 }), tiers: null });

    expect(screen.queryByRole('list', { name: 'Law' })).toBeNull();
    expect(textOf(screen.queryByText('No law activity yet.'))).toBe('No law activity yet.');
    const pending = screen.queryByRole('list', { name: 'Not counted yet' });
    const items = examined('pending counts', pending ? within(pending).queryAllByRole('listitem') : []);
    expect(items.map((item) => item.dataset.pending)).toEqual(['dues', 'mastery', 'leeches']);
    expect(textOf(document.body)).not.toMatch(/: 0/);
    // the tier distribution could not be read, and says so rather than showing zeros
    expect(screen.queryByRole('table')).toBeNull();
    expect(textOf(screen.queryByText(/by tier/))).toBe('The law cards by tier cannot be shown right now.');
  });

  it('lists only the counts that are pending', () => {
    render(LawBlock, { view: viewed({ dues: 3, lines: ['total_xp', 'dues'] }), tiers: TIERS });

    const pending = screen.queryByRole('list', { name: 'Not counted yet' });
    const items = examined('pending counts', pending ? within(pending).queryAllByRole('listitem') : []);
    expect(items.map(textOf)).toEqual(['Law mastery: pending', 'Active law leeches: pending']);
    expect(textOf(screen.queryByRole('heading', { level: 3 }))).toBe('Not counted yet');
  });

  it('shows the law cards and today\'s law XP by tier', () => {
    render(LawBlock, { view: viewed(), tiers: TIERS });

    const table = screen.queryByRole('table', { name: 'Law cards by tier' });
    const rows = examined('tier rows', table ? within(table).queryAllByRole('row') : []);
    expect(rows.map((row) => [...row.children].map(textOf))).toEqual([
      ['Tier', 'Cards', 'XP today'],
      ['T1', '40', '12'],
      ['T2', '31', '9'],
      ['T3', '18', '0'],
      ['T4', '5', '4'],
      ['No tier', '3', '0']
    ]);
    // each row is named by its tier
    expect(within(table as HTMLElement).queryAllByRole('rowheader').map(textOf)).toEqual(['T1', 'T2', 'T3', 'T4', 'No tier']);
    expect(rows.slice(1).map((row) => row.dataset.tier)).toEqual(['T1', 'T2', 'T3', 'T4', 'none']);
    expect(screen.queryByText(/cannot be shown/)).toBeNull();
  });
});

/** A synthetic distribution: today's law XP by the tier of each review's card. */
const TIERS: LawTiersView = {
  cards: { T1: 40, T2: 31, T3: 18, T4: 5, none: 3 },
  xpToday: { T1: 12, T2: 9, T3: 0, T4: 4, none: 0 }
};
