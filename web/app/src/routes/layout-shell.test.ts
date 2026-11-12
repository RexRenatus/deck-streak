/**
 * @vitest-environment jsdom
 */
import { render } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';

// SPEC-057 A24; SPEC-028 R2. The root layout tells Telegram the first screen is up, once, and
// renders the page it wraps.
const stub = vi.hoisted(() => {
  const calls = { ready: 0, expand: 0 };
  const inset = { top: 0, bottom: 0, left: 0, right: 0 };
  const noop = () => undefined;
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
        ready: () => {
          calls.ready += 1;
        },
        expand: () => {
          calls.expand += 1;
        },
        openLink: noop,
        onEvent: noop,
        offEvent: noop
      }
    }
  });
  return { calls };
});

import Layout from './+layout.svelte';

describe('the root layout', () => {
  it('calls ready and expand once when it mounts, and renders its page', () => {
    const children = createRawSnippet(() => ({ render: () => '<p>the page</p>' }));

    const { container } = render(Layout, { children });

    expect(container.textContent).toContain('the page');
    expect(stub.calls).toEqual({ ready: 1, expand: 1 });
  });
});
