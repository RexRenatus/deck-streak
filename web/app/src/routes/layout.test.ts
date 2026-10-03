/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Layout from './+layout.svelte';

// SPEC-084 R10. The root layout mounts the ladder on every screen: once the screen is on the page
// it tells Telegram the Mini App is ready and reads the owner's feed, and each served celebration
// appears in a region a screen reader announces, above the screen itself. The wallet's header
// (SPEC-082 R17) comes first, and its own tests judge it; here its read answers nothing. The feed's client and
// Telegram's wrapper are replaced before the layout's imports run.
const mocks = vi.hoisted(() => ({ ready: vi.fn(), feed: vi.fn(), wallet: vi.fn() }));
vi.mock('$lib/telegram.svelte', () => ({ telegram: { ready: mocks.ready } }));
vi.mock('$lib/api', () => ({ api: { feed: mocks.feed, wallet: mocks.wallet } }));

/** The layout around a synthetic screen. */
function mount() {
  mocks.wallet.mockResolvedValue({ kind: 'unavailable' });
  const children = createRawSnippet(() => ({ render: () => '<main>A synthetic screen</main>' }));
  return render(Layout, { props: { children } });
}

/** The tiers the ladder's region shows. */
function tiers(region: HTMLElement): (string | null)[] {
  return [...region.querySelectorAll('[data-tier]')].map((item) => item.getAttribute('data-tier'));
}

describe('the root layout', () => {
  it('announces each served celebration above the screen once it is ready', async () => {
    mocks.feed.mockResolvedValue({
      kind: 'ok',
      value: [
        { kind: 'celebration', text: 'A synthetic T4', tier: 'T4' },
        { kind: 'celebration', text: 'A synthetic T2', tier: 'T2' }
      ]
    });
    const { container } = mount();

    expect(mocks.ready).toHaveBeenCalledOnce();
    expect(mocks.feed).toHaveBeenCalledOnce();
    const region = screen.getByRole('region', { name: 'Celebrations' });
    expect(region.getAttribute('aria-live')).toBe('polite');
    await vi.waitFor(() => expect(tiers(region)).toEqual(['T4', 'T2']));
    expect(screen.getByText('A synthetic T4')).toBeTruthy();
    // the wallet's header, then the region, then the screen, which renders as it was given
    expect([...container.children].map((child) => child.tagName)).toEqual([
      'HEADER',
      'SECTION',
      'MAIN'
    ]);
    expect(screen.getByRole('main').textContent).toBe('A synthetic screen');
  });

  it('shows nothing in the region when the feed serves nothing', async () => {
    mocks.feed.mockResolvedValue({ kind: 'unavailable' });
    mount();

    const region = screen.getByRole('region', { name: 'Celebrations' });
    await vi.waitFor(() => expect(mocks.feed).toHaveBeenCalled());
    await Promise.resolve();
    expect(region.children).toHaveLength(0);
    expect(screen.getByRole('main').textContent).toBe('A synthetic screen');
  });
});
