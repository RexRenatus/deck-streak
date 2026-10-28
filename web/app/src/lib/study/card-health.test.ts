/**
 * @vitest-environment jsdom
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CardHealth, CardView, Head, HealthReadout, UndoOffer } from '$lib/engine/protocol';
import en from '../../../messages/en.json';
import { VERDICT } from './card-health';
import CardHealthPanel from './CardHealthPanel.svelte';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

// SPEC-507 A3 to A5; ADR-525. The card health panel shows the shown card's own record: its counters,
// its answers' capped times, its newest rows, the scheduler's labelled estimate only when the
// scheduler is Ready and the state's line otherwise, and where it came from. It judges no card, it
// reaches the record only through the review's read over the Worker, and the answer clock stops
// while it is open.
const APP = join(import.meta.dirname, '..', '..', '..');

/** Each state but Ready, and the line its recall rows read. */
const STATE_LINES: Record<Exclude<HealthReadout['state'], 'ready'>, string> = {
  absent: 'Not available in this build.',
  fixture: 'Not available: the scheduler is a test fixture.',
  unready: 'Not available: the scheduler has not read this card yet.',
  stale: "Not available: the scheduler's state is older than this card's last review."
};

/** A fixture card's record: 4 reviews, 1 lapse, the core's capped figures, and `scheduler`. */
function record(scheduler: HealthReadout, extra: Partial<CardHealth> = {}): CardHealth {
  return {
    card: 1_700_000_000_000n,
    reps: 4,
    lapses: 1,
    logged: { answers: 4, lapses: 1 },
    rows: 5,
    reviews: [
      { at: 1_700_000_005_000, grade: 3, kind: 2, interval: 1, ms: 20_000 },
      { at: 1_700_000_004_000, grade: 0, kind: 4, interval: 0, ms: 0 },
      { at: 1_700_000_003_000, grade: 1, kind: 1, interval: -600, ms: 12_000 },
      { at: 1_700_000_002_000, grade: 3, kind: 0, interval: 1, ms: 60_000 },
      { at: 1_700_000_001_000, grade: 3, kind: 0, interval: -600, ms: 8_000 }
    ],
    totalMs: 100_000,
    meanMs: 25_000,
    medianMs: 16_000,
    scheduler,
    provenance: { source: 'collection', createdMs: 1_700_000_000_000 },
    ...extra
  };
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The text of the element, its whitespace collapsed. */
function text(element: Element): string {
  return (element.textContent ?? '').replace(/\s+/g, ' ').trim();
}

function panel(health: CardHealth | null): void {
  render(CardHealthPanel, { health, loading: false, onclose: () => undefined });
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('the card health panel', () => {
  it("the recall rows read the state's line in every state but Ready", () => {
    for (const [state, line] of examined('state(s) but Ready', Object.entries(STATE_LINES))) {
      // a stray stored stability rides along on the record; the panel never shows it
      panel({ ...record({ state } as HealthReadout), stability: 12.34 } as CardHealth);
      const recall = screen.getByRole('region', { name: 'Recall' });
      expect(text(recall)).toBe(`Recall ${line}`);
      expect(text(recall)).not.toMatch(/\d/);
      expect(text(screen.getByRole('dialog'))).not.toContain('12.34');
      cleanup();
    }
    // Ready: the scheduler's two figures, under its label
    panel(record({ state: 'ready', recallPpm: 912_345, daysTo90: 12 }));
    expect(text(screen.getByRole('region', { name: 'Recall' }))).toBe(
      'Recall The scheduler estimates Chance of recall now: 91.2% Days until recall falls to 90%: 12'
    );
  });

  it('the panel shows the counts, the capped times and the newest reviews first', () => {
    panel(record({ state: 'absent' }));
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    expect(text(dialog)).toContain('Reviews 4 Lapses 1');
    expect(text(dialog)).toContain('Time answering, each answer at most 60 s 100.0 s');
    expect(text(dialog)).toContain('Mean answer time 25.0 s Median answer time 16.0 s');
    // each row's cells, in order: a table holds no whitespace between them
    const rows = screen
      .getAllByRole('row')
      .slice(1)
      .map((row) => [...row.querySelectorAll('td')].map(text).join(' '));
    expect(rows).toEqual([
      '2023-11-14 Good 20.0 s',
      '2023-11-14 Set by hand 0.0 s',
      '2023-11-14 Again 12.0 s',
      '2023-11-14 Good 60.0 s',
      '2023-11-14 Good 8.0 s'
    ]);
    // the counters agree with the log, so no line about the log
    expect(text(dialog)).not.toContain('The review log holds');
    cleanup();
    // a card whose counters differ from its log says what the log holds, as a fact
    panel(record({ state: 'absent' }, { reps: 0, lapses: 0 }));
    expect(text(screen.getByRole('dialog'))).toContain('The review log holds 4 answers and 1 lapses.');
  });

  it("the panel's strings label the estimate and name no verdict", () => {
    const strings = examined(
      'panel string(s)',
      Object.entries(en as Record<string, string>).filter(([key]) => key.startsWith('card_health_'))
    );
    expect((en as Record<string, string>).card_health_estimates).toBe('The scheduler estimates');
    expect(strings.filter(([, line]) => VERDICT.test(line))).toEqual([]);
    // each state's rendered panel holds no verdict either
    for (const state of [...Object.keys(STATE_LINES), 'ready']) {
      const readout = (state === 'ready' ? { state, recallPpm: 500_000, daysTo90: 3 } : { state }) as HealthReadout;
      panel(record(readout));
      const shown = text(screen.getByRole('dialog'))
        // the answer buttons' own names record what was pressed (SPEC-507 R8)
        .replace(/\b(Again|Hard|Good|Easy)\b/g, '');
      expect(VERDICT.test(shown), shown).toBe(false);
      cleanup();
    }
    // the census sees a planted verdict
    expect(['This card looks hard for you', 'a leech', 'at risk'].map((line) => VERDICT.test(line))).toEqual([
      true,
      true,
      true
    ]);
  });

  it('the panel says where the card came from', () => {
    panel(record({ state: 'absent' }));
    expect(text(screen.getByRole('region', { name: 'Where it came from' }))).toBe(
      'Where it came from Came through your collection, created 2023-11-14.'
    );
  });

  it("the panel's module opens no fetch and adds no route", () => {
    const NETWORK = /\bfetch\s*\(|XMLHttpRequest|WebSocket|EventSource|sendBeacon|\bgoto\s*\(|\bhref=/;
    const sources = examined('panel source(s)', [
      'src/lib/study/CardHealthPanel.svelte',
      'src/lib/study/card-health.ts'
    ]);
    for (const source of sources) {
      expect(NETWORK.test(readFileSync(join(APP, source), 'utf8')), source).toBe(false);
    }
    expect(NETWORK.test("const reply = await fetch('/api/card');")).toBe(true);
    // the record comes only through the review's read, which the client sends as one Worker operation
    const screenSource = readFileSync(join(APP, 'src/lib/study/ReviewScreen.svelte'), 'utf8');
    expect(screenSource).toMatch(/<CardHealthPanel\b/);
    expect(screenSource).toMatch(/review\.cardHealth\(\)/);
    expect(readFileSync(join(APP, 'src/lib/engine/client.ts'), 'utf8')).toMatch(/op: 'card-health'/);
    // no screen under src/routes is the panel's
    const routes = readdirSync(join(APP, 'src/routes'), { recursive: true, encoding: 'utf8' });
    expect(examined('route path(s)', routes).filter((path) => /health|card-info/i.test(path))).toEqual([]);
  });
});

// ---- the panel on the review screen ----

function view(id: number): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false
  };
}

/** The engine as the screen sees it, with the card health read. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  async card(): Promise<Head> {
    this.calls.push('card');
    return this.heads.shift() as Head;
  }

  async rate(card: bigint, rating: number, ms: number): Promise<null> {
    this.calls.push(`rate ${card} ${rating} ${ms}`);
    return null;
  }

  async bury(): Promise<null> {
    return null;
  }

  async flag(): Promise<number> {
    return 0;
  }

  async undo(): Promise<null> {
    return null;
  }

  async undoOffer(): Promise<UndoOffer> {
    return { offer: null, why: 'none' };
  }

  async cardHealth(card: bigint): Promise<CardHealth | null> {
    this.calls.push(`card-health ${card}`);
    return record({ state: 'absent' }, { card });
  }
}

let now = 0;

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

describe('the card health panel on the review', () => {
  beforeEach(() => {
    now = 1000;
    vi.spyOn(performance, 'now').mockImplementation(() => now);
    Object.defineProperty(navigator, 'wakeLock', {
      configurable: true,
      value: { request: () => new Promise(() => undefined) }
    });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'visible' });
    localStorage.clear();
  });

  it('the card info panel pauses the answer clock', async () => {
    const counts = { new: 1, learning: 0, review: 0 };
    const client = new FakeClient([
      { counts, card: view(1) },
      { counts, card: null }
    ]);
    render(ReviewScreen, { client: async () => client });
    await settle();
    const region = screen.getByRole('region', { name: 'Review' });

    // 5 s on the question, then the panel opens over it for 30 s
    now += 5_000;
    await fireEvent.click(screen.getByRole('button', { name: 'Card info' }));
    await settle();
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    expect(dialog.contains(document.activeElement)).toBe(true);
    expect(screen.getByRole('region', { name: 'Recall' }).textContent).toContain('Not available in this build.');
    now += 30_000;
    // a key the review would take does nothing while the panel is open
    await fireEvent.keyDown(window, { key: ' ' });
    expect(screen.queryByRole('button', { name: 'Show answer' })).not.toBeNull();

    // Escape closes it, and the focus returns to the review
    await fireEvent.keyDown(window, { key: 'Escape' });
    await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(region);

    // 2 s more on the card: the grade sends 7 s, not 37 s, and names the card shown
    now += 2_000;
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect(client.calls).toEqual(['card', 'card-health 1', 'rate 1 3 7000', 'card']);
  });

  it('Tab and Shift+Tab stay inside the open panel', async () => {
    const client = new FakeClient([{ counts: { new: 1, learning: 0, review: 0 }, card: view(1) }]);
    render(ReviewScreen, { client: async () => client });
    await settle();
    await fireEvent.click(screen.getByRole('button', { name: 'Card info' }));
    await settle();
    const dialog = screen.getByRole('dialog', { name: 'Card info' });
    const close = screen.getByRole('button', { name: 'Close' });
    expect(document.activeElement).toBe(close);
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(dialog.contains(document.activeElement)).toBe(true);
    await fireEvent.keyDown(window, { key: 'Tab', shiftKey: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});
