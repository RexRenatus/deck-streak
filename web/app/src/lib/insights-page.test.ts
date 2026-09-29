/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Answer } from './api';
import type { Envelope, Listing } from './insights/insights';

// SPEC-094 R19. The insights screen shows each instrument's latest stored report exactly as the
// server answered, and a failed or unopened read as a line that says so.
const insights = vi.fn<() => Promise<Answer<Listing[]>>>();
const insight = vi.fn<(id: string) => Promise<Answer<Envelope | null>>>();
vi.mock('$lib/api', () => ({ api: { insights: () => insights(), insight: (id: string) => insight(id) } }));

import Page from '../routes/insights/+page.svelte';

const LISTING: Listing = { id: 'dark_fields', cadence: 'weekly', studyDay: 5 };
const STORED = {
  dark_fields: [{ note_type: 'Type A', field: 'Extra', reviewed_notes: 7 }],
  dark_fields_total: 1,
  unparseable: [],
  unparseable_total: 0,
  notetypes_checked: 2,
  reviewed_note_count: 30,
  is_cold: false
};

function envelope(over: Partial<Envelope> = {}): Envelope {
  return { instrument: 'dark_fields', studyDay: 5, failedReads: [], report: STORED, ...over };
}

beforeEach(() => {
  insights.mockReset();
  insight.mockReset();
});

describe('the insights screen', () => {
  it('shows a loading line until the listing arrives', async () => {
    insights.mockReturnValue(new Promise(() => {}));
    render(Page);
    expect(screen.getByRole('status').textContent).toContain('Loading');
    expect(screen.getByRole('heading', { name: 'Insights' })).toBeTruthy();
  });

  it('asks the owner to reopen when the session is refused', async () => {
    insights.mockResolvedValue({ kind: 'reopen' });
    render(Page);
    expect((await screen.findByRole('alert')).textContent).toContain('Reopen DeckStreak');
    expect(insight).not.toHaveBeenCalled();
  });

  it('says the server is unavailable when the listing fails', async () => {
    insights.mockResolvedValue({ kind: 'unavailable' });
    render(Page);
    expect((await screen.findByRole('alert')).textContent).toContain('could not reach');
    expect(insight).not.toHaveBeenCalled();
  });

  it('says no instrument has reported when the listing is empty', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [] });
    render(Page);
    expect(await screen.findByText('No instrument has reported yet.')).toBeTruthy();
  });

  it('shows a loading line for an instrument whose report has not arrived', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockReturnValue(new Promise(() => {}));
    render(Page);
    await waitFor(() => expect(insight).toHaveBeenCalledWith('dark_fields'));
    expect(screen.getByRole('status').textContent).toBe('Loading…');
  });

  it('draws the Dark Fields section from a stored report', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'ok', value: envelope() });
    render(Page);
    expect(await screen.findByRole('heading', { name: 'Dark fields' })).toBeTruthy();
    expect(screen.getByRole('listitem').textContent).toContain('Type A');
  });

  it('says an instrument has not run when its report is null', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'ok', value: null });
    render(Page);
    expect(await screen.findByText('This instrument has not run yet.')).toBeTruthy();
  });

  it('asks to reopen when one instrument read is refused', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'reopen' });
    render(Page);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Reopen DeckStreak'));
  });

  it('says the server is unavailable when one instrument read fails', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'unavailable' });
    render(Page);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('could not reach'));
  });

  it('says the server is unavailable when the stored report is not Dark Fields', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'ok', value: envelope({ report: { nonsense: true } }) });
    render(Page);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('could not reach'));
    expect(screen.getByRole('heading', { name: 'Dark fields' })).toBeTruthy();
  });

  it('draws nothing for an instrument it has no view for, and links back to today', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [{ ...LISTING, id: 'other' }] });
    insight.mockResolvedValue({ kind: 'ok', value: envelope({ instrument: 'other' }) });
    render(Page);
    await waitFor(() => expect(insight).toHaveBeenCalledWith('other'));
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    expect(screen.queryByRole('heading', { name: 'Dark fields' })).toBeNull();
    expect(screen.getByRole('link').getAttribute('href')).toBe('/');
    expect(screen.getByRole('link').textContent).toBe('Back to Today');
  });

  it('draws a failed run as a failure line inside its section', async () => {
    insights.mockResolvedValue({ kind: 'ok', value: [LISTING] });
    insight.mockResolvedValue({ kind: 'ok', value: envelope({ failedReads: ['templates'] }) });
    render(Page);
    expect((await screen.findByRole('alert')).textContent).toContain('templates');
  });
});
