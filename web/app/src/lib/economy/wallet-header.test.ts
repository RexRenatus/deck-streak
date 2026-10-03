/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Layout from '../../routes/+layout.svelte';

// SPEC-082 A18, R17. The root layout shows the owner's balance in a header on every screen, above
// the ladder's region and the screen itself, as a link to the wallet's history. The wallet's
// client and Telegram's wrapper are replaced before the layout's imports run; every number is
// synthetic.
const mocks = vi.hoisted(() => ({ ready: vi.fn(), feed: vi.fn(), wallet: vi.fn() }));
vi.mock('$lib/telegram.svelte', () => ({ telegram: { ready: mocks.ready } }));
vi.mock('$lib/api', () => ({ api: { feed: mocks.feed, wallet: mocks.wallet } }));

const WALLET = {
  studyDay: '2025-01-14',
  balance: 103,
  lossCap: 30,
  lossCapLeft: 25,
  movements: [],
  next: null
};

/** The layout around a synthetic screen. */
function mount() {
  mocks.feed.mockResolvedValue({ kind: 'unavailable' });
  const children = createRawSnippet(() => ({ render: () => '<main>A synthetic screen</main>' }));
  return render(Layout, { props: { children } });
}

describe('the wallet header', () => {
  it('shows the balance in the layout header', async () => {
    mocks.wallet.mockReset();
    mocks.wallet.mockResolvedValue({ kind: 'ok', value: WALLET });
    const { container } = mount();

    const link = await screen.findByRole('link', { name: 'Coins: 103' });
    expect(link.getAttribute('href')).toBe('/wallet');
    // the header is the layout's first child, above the ladder's region and the screen
    expect(link.closest('header')).toBe(container.firstElementChild);
    // the first page, read once
    expect(mocks.wallet).toHaveBeenCalledOnce();
    expect(mocks.wallet).toHaveBeenCalledWith();
  });

  it('links to the wallet by name while no balance has arrived', async () => {
    mocks.wallet.mockReset();
    mocks.wallet.mockResolvedValue({ kind: 'unavailable' });
    const { container } = mount();

    await vi.waitFor(() => expect(mocks.wallet).toHaveBeenCalledOnce());
    const link = screen.getByRole('link', { name: 'Wallet' });
    expect(link.getAttribute('href')).toBe('/wallet');
    expect(link.closest('header')).toBe(container.firstElementChild);
    expect(screen.queryByText(/Coins:/)).toBeNull();
  });
});
