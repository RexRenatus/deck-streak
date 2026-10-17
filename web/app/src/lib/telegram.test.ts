/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, vi } from 'vitest';
import type { Insets } from './telegram.svelte';

// The module is imported inside each test. It builds the app's wrapper when it loads, so a
// mutant of that construction fails the test that loads it, where a static import would fail the
// whole file before any test ran.
const loaded = () => import('./telegram.svelte');

// SPEC-028 R2, A7. Telegram's script throws WebAppMethodUnsupported when a method is called on a
// client older than the Bot API version that added it. The versions below are read from
// telegram.org/js/telegram-web-app.js itself; the stub throws exactly as it does, so a wrapper
// call made without its isVersionAtLeast gate fails this matrix instead of breaking an old client.
const THROWS_BELOW: Readonly<Record<string, string>> = {
  invokeCustomMethod: '6.9',
  requestFullscreen: '8.0',
  exitFullscreen: '8.0',
  addToHomeScreen: '8.0',
  checkHomeScreenStatus: '8.0',
  switchInlineQuery: '6.6',
  openInvoice: '6.1',
  showPopup: '6.2',
  showAlert: '6.2',
  showConfirm: '6.2',
  showScanQrPopup: '6.4',
  closeScanQrPopup: '6.4',
  readTextFromClipboard: '6.4',
  requestWriteAccess: '6.9',
  requestContact: '6.9',
  downloadFile: '8.0',
  shareToStory: '7.8',
  shareMessage: '8.0',
  requestChat: '9.6',
  setEmojiStatus: '8.0',
  requestEmojiStatusAccess: '8.0'
};
// The storage objects throw from every method below their version.
const STORAGE_BELOW: Readonly<Record<string, string>> = {
  CloudStorage: '6.9',
  DeviceStorage: '9.0',
  SecureStorage: '9.0'
};

const NOTCH: Insets = { top: 47, bottom: 34, left: 0, right: 0 };
const HEADER: Insets = { top: 56, bottom: 0, left: 0, right: 0 };

function atLeast(version: string, floor: string): boolean {
  const have = version.split('.').map(Number);
  const need = floor.split('.').map(Number);
  for (let i = 0; i < Math.max(have.length, need.length); i++) {
    const difference = (have[i] ?? 0) - (need[i] ?? 0);
    if (difference !== 0) return difference > 0;
  }
  return true;
}

/** A stub of Telegram's Mini App object at `version`, counting calls and raising events on demand. */
function stubAt(version: string) {
  const calls: Record<string, number> = {};
  const count = (name: string) => {
    calls[name] = (calls[name] ?? 0) + 1;
  };
  const handlers = new Map<string, Array<(payload?: { isStateStable?: boolean }) => void>>();
  const unsupported: string[] = [];
  const gated = (name: string, floor: string) => () => {
    if (!atLeast(version, floor)) {
      unsupported.push(name);
      throw new Error('WebAppMethodUnsupported');
    }
    count(name);
  };
  const storage = (name: string, floor: string) =>
    Object.fromEntries(
      ['setItem', 'getItem', 'getItems', 'removeItem', 'removeItems', 'getKeys', 'clear'].map(
        (method) => [method, gated(`${name}.${method}`, floor)]
      )
    );
  const webApp = {
    initData: 'auth_date=1&start_param=about&hash=synthetic',
    version,
    platform: 'tdesktop',
    colorScheme: 'light' as 'light' | 'dark',
    themeParams: { bg_color: '#ffffff', text_color: '#000000' },
    viewportStableHeight: 640,
    safeAreaInset: NOTCH,
    contentSafeAreaInset: HEADER,
    isVersionAtLeast: (floor: string) => atLeast(version, floor),
    ready: () => count('ready'),
    expand: () => count('expand'),
    openLink: vi.fn(),
    onEvent: (event: string, handler: (payload?: { isStateStable?: boolean }) => void) => {
      handlers.set(event, [...(handlers.get(event) ?? []), handler]);
    },
    offEvent: () => undefined,
    ...Object.fromEntries(Object.entries(THROWS_BELOW).map(([name, floor]) => [name, gated(name, floor)])),
    ...Object.fromEntries(
      Object.entries(STORAGE_BELOW).map(([name, floor]) => [name, storage(name, floor)])
    )
  };
  const emit = (event: string, payload?: { isStateStable?: boolean }) => {
    for (const handler of handlers.get(event) ?? []) handler(payload);
  };
  return { webApp, calls, unsupported, emit };
}

describe('the one Telegram wrapper', () => {
  it('the wrapper calls ready once and gates each method by version', async () => {
    const { connect, NO_INSETS } = await loaded();
    // the first Mini App version, the versions that added what the wrapper gates, and the newest
    const matrix = ['6.0', '6.9', '7.10', '8.0', '9.6'];
    const outcomes = matrix.map((version) => {
      const stub = stubAt(version);
      const wrapper = connect({ Telegram: { WebApp: stub.webApp } });

      wrapper.ready();
      wrapper.ready();
      wrapper.openLink('https://example.org/');
      stub.emit('themeChanged');
      stub.emit('viewportChanged', { isStateStable: true });
      stub.emit('safeAreaChanged');
      stub.emit('contentSafeAreaChanged');

      return {
        version,
        ready: stub.calls.ready ?? 0,
        expand: stub.calls.expand ?? 0,
        unsupported: stub.unsupported,
        safeArea: wrapper.safeAreaInset,
        contentSafeArea: wrapper.contentSafeAreaInset
      };
    });

    expect(outcomes).toEqual(
      matrix.map((version) => ({
        version,
        ready: 1,
        expand: 1,
        unsupported: [],
        // Bot API 8.0 added safe areas: below it the wrapper reports none
        safeArea: atLeast(version, '8.0') ? NOTCH : NO_INSETS,
        contentSafeArea: atLeast(version, '8.0') ? HEADER : NO_INSETS
      }))
    );
  });

  it('the wrapper reads the launch once and follows the theme and the viewport', async () => {
    const { connect, NO_INSETS } = await loaded();
    const stub = stubAt('9.6');
    const wrapper = connect({ Telegram: { WebApp: stub.webApp } });

    expect(wrapper.inside).toBe(true);
    expect([wrapper.platform, wrapper.version, wrapper.startParam]).toEqual(['tdesktop', '9.6', 'about']);
    expect(wrapper.launchData).toBe(stub.webApp.initData);
    expect([wrapper.colorScheme, wrapper.themeParams.bg_color]).toEqual(['light', '#ffffff']);

    stub.webApp.colorScheme = 'dark';
    stub.webApp.themeParams = { bg_color: '#212121', text_color: '#ffffff' };
    stub.webApp.viewportStableHeight = 480;
    stub.emit('themeChanged');
    // a viewport still moving is not yet the stable height
    stub.emit('viewportChanged', { isStateStable: false });
    expect(wrapper.viewportStableHeight).toBe(640);
    stub.emit('viewportChanged', { isStateStable: true });

    expect([wrapper.colorScheme, wrapper.themeParams.bg_color, wrapper.viewportStableHeight]).toEqual([
      'dark',
      '#212121',
      480
    ]);
    expect(wrapper.openLink('https://example.org/a')).toBe(true);
    expect(stub.webApp.openLink).toHaveBeenCalledWith('https://example.org/a');
  });

  it('outside Telegram the wrapper says so and never throws', async () => {
    const { connect, NO_INSETS } = await loaded();
    for (const host of [undefined, {}, { Telegram: {} }]) {
      const wrapper = connect(host);

      wrapper.ready();

      expect([wrapper.inside, wrapper.launchData, wrapper.startParam]).toEqual([false, null, null]);
      expect(wrapper.openLink('https://example.org/')).toBe(false);
      expect(wrapper.safeAreaInset).toEqual(NO_INSETS);
    }
  });
  it('outside Telegram the wrapper reports the defaults of a page that is not in a client', async () => {
    const { connect, NO_INSETS } = await loaded();
    for (const host of [undefined, {}, { Telegram: {} }]) {
      const wrapper = connect(host);

      expect(wrapper.platform).toBe('unknown');
      expect(wrapper.version).toBe('');
      expect(wrapper.colorScheme).toBe('light');
      expect(wrapper.themeParams).toEqual({});
      expect(wrapper.viewportStableHeight).toBeNull();
      expect(wrapper.contentSafeAreaInset).toEqual({ top: 0, bottom: 0, left: 0, right: 0 });
    }
    expect(NO_INSETS).toEqual({ top: 0, bottom: 0, left: 0, right: 0 });
    expect(Object.isFrozen(NO_INSETS)).toBe(true);
  });

  it('a launch with no data has no start parameter, and a launch without one has none either', async () => {
    const { connect, NO_INSETS } = await loaded();
    const empty = stubAt('9.6');
    empty.webApp.initData = '';
    const bare = stubAt('9.6');
    bare.webApp.initData = 'auth_date=1&hash=synthetic';

    const first = connect({ Telegram: { WebApp: empty.webApp } });
    const second = connect({ Telegram: { WebApp: bare.webApp } });

    expect([first.inside, first.launchData, first.startParam]).toEqual([true, null, null]);
    expect([second.launchData, second.startParam]).toEqual([bare.webApp.initData, null]);
  });

  it('a client that sends no platform or version leaves the defaults', async () => {
    const { connect, NO_INSETS } = await loaded();
    const stub = stubAt('9.6');
    const webApp = { ...stub.webApp, platform: undefined, version: undefined } as unknown as typeof stub.webApp;

    const wrapper = connect({ Telegram: { WebApp: webApp } });

    expect([wrapper.platform, wrapper.version]).toEqual(['unknown', '']);
  });

  it('a viewport change with no payload is ignored', async () => {
    const { connect, NO_INSETS } = await loaded();
    const stub = stubAt('9.6');
    const wrapper = connect({ Telegram: { WebApp: stub.webApp } });
    stub.webApp.viewportStableHeight = 300;

    expect(() => stub.emit('viewportChanged')).not.toThrow();

    expect(wrapper.viewportStableHeight).toBe(640);
  });

  it('the safe areas follow their own events, each on its own field', async () => {
    const { connect, NO_INSETS } = await loaded();
    const stub = stubAt('9.6');
    const wrapper = connect({ Telegram: { WebApp: stub.webApp } });
    const moved: Insets = { top: 10, bottom: 20, left: 30, right: 40 };
    const covered: Insets = { top: 1, bottom: 2, left: 3, right: 4 };

    stub.webApp.safeAreaInset = moved;
    stub.webApp.contentSafeAreaInset = covered;
    stub.emit('safeAreaChanged');
    expect(wrapper.safeAreaInset).toEqual(moved);
    expect(wrapper.contentSafeAreaInset).toEqual(HEADER);
    stub.emit('contentSafeAreaChanged');

    expect(wrapper.contentSafeAreaInset).toEqual(covered);
    expect(wrapper.safeAreaInset).toEqual(moved);
  });

  it('the app wrapper reads the page it runs in', async () => {
    vi.resetModules();
    const stub = stubAt('9.6');
    vi.stubGlobal('Telegram', { WebApp: stub.webApp });
    try {
      const fresh = await import('./telegram.svelte');
      expect(fresh.telegram.inside).toBe(true);
      expect(fresh.telegram.startParam).toBe('about');
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
