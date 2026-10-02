/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Page from '../../routes/wallet/+page.svelte';
import { parseWallet, signed, walletPath, type WalletView } from './wallet';

// SPEC-082 A20, R15, R17; ADR-315 ruling 3. The wallet's screen lists the coin movements in the
// server's order, newest first, and appends the older page the server names when the owner asks.
// The client is replaced before the page's imports run; every movement is synthetic.
const mocks = vi.hoisted(() => ({ wallet: vi.fn() }));
vi.mock('$lib/api', () => ({ api: { wallet: mocks.wallet } }));

const FIRST: WalletView = {
  studyDay: '2025-01-14',
  balance: 103,
  lossCap: 30,
  lossCapLeft: 25,
  movements: [
    { id: 4, studyDay: '2025-01-14', source: 'fine', amount: -5 },
    { id: 3, studyDay: '2025-01-14', source: 'mint', amount: 8 },
    { id: 2, studyDay: '2025-01-13', source: 'mint', amount: 40 }
  ],
  next: 2
};
const OLDER: WalletView = {
  ...FIRST,
  movements: [{ id: 1, studyDay: '2025-01-12', source: 'payout', amount: 60 }],
  next: null
};

/** The movements the screen lists: each line's movement id, in the order shown. */
function shown(container: HTMLElement): (string | null)[] {
  return [...container.querySelectorAll('li[data-movement-id]')].map((line) =>
    line.getAttribute('data-movement-id')
  );
}

/** Each line's text, its whitespace collapsed. */
function lines(container: HTMLElement): string[] {
  return [...container.querySelectorAll('li[data-movement-id]')].map((line) =>
    (line.textContent ?? '').replace(/\s+/g, ' ').trim()
  );
}

describe('the wallet history', () => {
  it('lists the movements newest first', async () => {
    mocks.wallet.mockReset();
    mocks.wallet.mockImplementation((before?: number) =>
      Promise.resolve({ kind: 'ok', value: before === undefined ? FIRST : OLDER })
    );
    const { container } = render(Page);

    await vi.waitFor(() => expect(shown(container)).toEqual(['4', '3', '2']));
    expect(mocks.wallet).toHaveBeenCalledWith();
    expect(screen.getByText('Coins: 103')).toBeTruthy();
    expect(screen.getByText("Today's loss limit: 25 of 30 left")).toBeTruthy();
    // each line: its study day, its source in plain words (or its code), and its signed amount
    expect(lines(container)).toEqual([
      '2025-01-14 fine -5',
      '2025-01-14 Coins earned by study +8',
      '2025-01-13 Coins earned by study +40'
    ]);

    await fireEvent.click(screen.getByRole('button', { name: 'Show older movements' }));
    await vi.waitFor(() => expect(shown(container)).toEqual(['4', '3', '2', '1']));
    expect(mocks.wallet).toHaveBeenLastCalledWith(2);
    expect(mocks.wallet).toHaveBeenCalledTimes(2);
    // no page is older than the last one
    expect(screen.queryByRole('button', { name: 'Show older movements' })).toBeNull();
  });

  it('says so when there is no movement yet', async () => {
    mocks.wallet.mockReset();
    mocks.wallet.mockResolvedValue({
      kind: 'ok',
      value: { ...FIRST, balance: 0, movements: [], next: null }
    });
    const { container } = render(Page);

    expect(await screen.findByText('No coin movements yet.')).toBeTruthy();
    expect(shown(container)).toEqual([]);
    expect(screen.queryByRole('button', { name: 'Show older movements' })).toBeNull();
  });

  it('keeps the lines it has when an older page does not arrive', async () => {
    mocks.wallet.mockReset();
    mocks.wallet.mockImplementation((before?: number) =>
      Promise.resolve(before === undefined ? { kind: 'ok', value: FIRST } : { kind: 'unavailable' })
    );
    const { container } = render(Page);

    await vi.waitFor(() => expect(shown(container)).toEqual(['4', '3', '2']));
    await fireEvent.click(screen.getByRole('button', { name: 'Show older movements' }));
    expect(await screen.findByRole('alert')).toBeTruthy();
    expect(shown(container)).toEqual(['4', '3', '2']);
  });
});

describe('the wallet body', () => {
  const BODY = {
    study_day: '2025-01-14',
    balance: 103,
    loss_cap: 30,
    loss_cap_left: 25,
    movements: [
      { id: 4, study_day: '2025-01-14', source: 'fine', amount: -5 },
      { id: 2, study_day: '2025-01-13', source: 'mint', amount: 40 }
    ],
    next: 2
  };

  it('reads the server body into the view, in its order', () => {
    expect(parseWallet(BODY)).toStrictEqual({
      studyDay: '2025-01-14',
      balance: 103,
      lossCap: 30,
      lossCapLeft: 25,
      movements: [
        { id: 4, studyDay: '2025-01-14', source: 'fine', amount: -5 },
        { id: 2, studyDay: '2025-01-13', source: 'mint', amount: 40 }
      ],
      next: 2
    });
    expect(parseWallet({ ...BODY, next: null })?.next).toBeNull();
  });

  it('refuses a body that is not the wallet', () => {
    const broken: unknown[] = [
      null,
      'wallet',
      { ...BODY, study_day: '14 January' },
      { ...BODY, balance: '103' },
      { ...BODY, balance: 10.5 },
      { ...BODY, loss_cap: undefined },
      { ...BODY, loss_cap_left: null },
      { ...BODY, movements: 'none' },
      { ...BODY, movements: [{ ...BODY.movements[0], id: '4' }] },
      { ...BODY, movements: [{ ...BODY.movements[0], study_day: '' }] },
      { ...BODY, movements: [{ ...BODY.movements[0], source: 5 }] },
      { ...BODY, movements: [{ ...BODY.movements[0], amount: 1.5 }] },
      { ...BODY, next: '2' },
      { ...BODY, next: undefined }
    ];
    for (const body of broken) {
      expect(parseWallet(body), JSON.stringify(body)).toBeNull();
    }
  });

  it('names the first page, and the page after a movement', () => {
    expect(walletPath()).toBe('/api/wallet');
    expect(walletPath(3)).toBe('/api/wallet?before=3');
  });

  it('the client reads the first page, then the page the cursor names', async () => {
    // the real client, past this file's stand-in, over a server that records what it was sent
    const { createApi } = await vi.importActual<typeof import('$lib/api')>('$lib/api');
    const sent: string[] = [];
    const fetch = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
      const url = String(input);
      sent.push(`${init.method ?? 'GET'} ${url}`);
      if (url === '/api/session') return new Response(null, { status: 204 });
      return Response.json(BODY);
    });
    const client = createApi({
      launchData: () => 'auth_date=1&hash=synthetic-launch-data',
      fetch: fetch as unknown as typeof globalThis.fetch
    });

    expect(await client.wallet()).toStrictEqual({ kind: 'ok', value: parseWallet(BODY) });
    expect((await client.wallet(2)).kind).toBe('ok');
    expect(sent).toEqual(['POST /api/session', 'GET /api/wallet', 'GET /api/wallet?before=2']);
  });

  it('shows a deposit with its plus and a debit with its minus', () => {
    expect([8, -5, 0].map(signed)).toEqual(['+8', '-5', '0']);
  });
});
