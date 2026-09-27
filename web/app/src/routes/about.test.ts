/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import About from './about/+page.svelte';

// SPEC-028 R9, A12; ADR-018. About offers the privacy policy and the source code (the AGPL's
// network-use offer). Inside Telegram a link opens through the wrapper's openLink, in the
// browser, rather than navigating the Mini App away from itself.
const telegram = vi.hoisted(() => {
  const noop = () => undefined;
  const openLink = vi.fn();
  const inset = { top: 0, bottom: 0, left: 0, right: 0 };
  Object.assign(globalThis, {
    Telegram: {
      WebApp: {
        initData: 'auth_date=1&hash=synthetic',
        version: '9.0',
        platform: 'tdesktop',
        colorScheme: 'light',
        themeParams: {},
        viewportStableHeight: 640,
        safeAreaInset: inset,
        contentSafeAreaInset: inset,
        isVersionAtLeast: () => true,
        ready: noop,
        expand: noop,
        openLink,
        onEvent: noop,
        offEvent: noop
      }
    }
  });
  return { openLink };
});

const PRIVACY_POLICY = 'https://github.com/RexRenatus/deck-streak/blob/main/PRIVACY.md';
const SOURCE_CODE = 'https://github.com/RexRenatus/deck-streak';

describe('About', () => {
  it('About links the privacy policy and the source code', async () => {
    render(About);

    const privacy = screen.getByRole('link', { name: 'Privacy policy' });
    const source = screen.getByRole('link', { name: 'Source code' });
    expect([privacy.getAttribute('href'), source.getAttribute('href')]).toEqual([
      PRIVACY_POLICY,
      SOURCE_CODE
    ]);

    // each opens through Telegram, and the Mini App stays where it is
    expect([await fireEvent.click(privacy), await fireEvent.click(source)]).toEqual([false, false]);
    expect(telegram.openLink.mock.calls).toEqual([[PRIVACY_POLICY], [SOURCE_CODE]]);
  });
});
