/**
 * The one wrapper around Telegram's Mini App script (ADR-005; SPEC-028 R2, R4).
 *
 * On a launch, `telegram-web-app.js` runs before any app code: the app's start hook adds it to the
 * head only when Telegram launched the page and waits for it (`telegram-launch.ts`, SPEC-400). It
 * parses the launch parameters out of the URL hash, keeps them for the session, and sets the
 * `--tg-*` CSS variables the design tokens read. This module is the only one that reads the
 * script's object (`telegram-boundary.test.ts` holds every other module to that). It reads the
 * launch once, when the module loads, after the script has run; it exposes what the screens need
 * as runes state; it gates a newer method behind `isVersionAtLeast`, as the script throws on an
 * older client. Outside a launch (a plain browser tab) it says so, and nothing it offers throws.
 */

/** Telegram's colour scheme for the chat the Mini App opened from. */
export type ColorScheme = 'light' | 'dark';

/** A safe-area inset in CSS pixels, per edge. */
export interface Insets {
  readonly top: number;
  readonly bottom: number;
  readonly left: number;
  readonly right: number;
}

/** Telegram's theme parameters, keyed as Telegram names them (`bg_color`, `text_color`, ...). */
export type ThemeParams = Readonly<Record<string, string>>;

/** The part of Telegram's Mini App object this wrapper reads; Telegram's script provides it. */
export interface WebApp {
  readonly initData: string;
  readonly version: string;
  readonly platform: string;
  readonly colorScheme: ColorScheme;
  readonly themeParams: ThemeParams;
  readonly viewportStableHeight: number;
  readonly safeAreaInset: Insets;
  readonly contentSafeAreaInset: Insets;
  isVersionAtLeast(version: string): boolean;
  ready(): void;
  expand(): void;
  openLink(url: string): void;
  onEvent(event: string, handler: (payload?: { readonly isStateStable?: boolean }) => void): void;
}

/** What the wrapper needs of the page's global object; a test hands it a stub. */
export interface Host {
  readonly Telegram?: { readonly WebApp?: WebApp };
}

/** No inset on any edge. */
export const NO_INSETS: Insets = Object.freeze({ top: 0, bottom: 0, left: 0, right: 0 });

// Bot API 8.0 added the safe areas and their events. An older client has none to report.
const SAFE_AREAS = '8.0';

function copy(insets: Insets): Insets {
  return { top: insets.top, bottom: insets.bottom, left: insets.left, right: insets.right };
}

/** The one wrapper around Telegram's Mini App object. */
export class TelegramWrapper {
  /** Whether the page runs inside Telegram. */
  readonly inside: boolean;
  /** The client's platform, such as `ios` or `tdesktop`; `unknown` outside Telegram. */
  readonly platform: string;
  /** The Bot API version the client supports; empty outside Telegram. */
  readonly version: string;
  /**
   * The startapp token of the link that launched the app, as the signed launch data carries it,
   * or null. Anyone can write a token into a link, so `startapp.ts` maps it onto the route table
   * and nothing else.
   */
  readonly startParam: string | null;
  /** The raw, signed launch data, which `api.ts` posts once to open a session; null outside Telegram. */
  readonly launchData: string | null;
  /** The chat's colour scheme; it follows Telegram's theme changes. */
  colorScheme = $state<ColorScheme>('light');
  /** The chat's theme parameters; the design tokens read the same colours as CSS variables. */
  themeParams = $state<ThemeParams>({});
  /** The height the Mini App keeps once the client stops resizing it; null outside Telegram. */
  viewportStableHeight = $state<number | null>(null);
  /** The device's safe area (a notch, a home indicator), from Bot API 8.0. */
  safeAreaInset = $state<Insets>(NO_INSETS);
  /** The area Telegram's own controls cover in full screen, from Bot API 8.0. */
  contentSafeAreaInset = $state<Insets>(NO_INSETS);
  readonly #webApp: WebApp | undefined;
  #ready = false;

  constructor(host: Host | undefined) {
    const webApp = host?.Telegram?.WebApp;
    this.#webApp = webApp;
    this.inside = webApp !== undefined;
    this.platform = webApp?.platform ?? 'unknown';
    this.version = webApp?.version ?? '';
    this.launchData = webApp?.initData ? webApp.initData : null;
    this.startParam =
      this.launchData === null ? null : new URLSearchParams(this.launchData).get('start_param');
    if (webApp === undefined) return;

    this.colorScheme = webApp.colorScheme;
    this.themeParams = { ...webApp.themeParams };
    this.viewportStableHeight = webApp.viewportStableHeight;
    webApp.onEvent('themeChanged', () => {
      this.colorScheme = webApp.colorScheme;
      this.themeParams = { ...webApp.themeParams };
    });
    webApp.onEvent('viewportChanged', (payload) => {
      if (payload?.isStateStable) this.viewportStableHeight = webApp.viewportStableHeight;
    });
    if (webApp.isVersionAtLeast(SAFE_AREAS)) {
      this.safeAreaInset = copy(webApp.safeAreaInset);
      this.contentSafeAreaInset = copy(webApp.contentSafeAreaInset);
      webApp.onEvent('safeAreaChanged', () => {
        this.safeAreaInset = copy(webApp.safeAreaInset);
      });
      webApp.onEvent('contentSafeAreaChanged', () => {
        this.contentSafeAreaInset = copy(webApp.contentSafeAreaInset);
      });
    }
  }

  /**
   * Tells Telegram the first screen has rendered, so it drops its own placeholder, and expands
   * the Mini App to its full height. Only the first call acts.
   */
  ready(): void {
    if (this.#ready || this.#webApp === undefined) return;
    this.#ready = true;
    this.#webApp.ready();
    this.#webApp.expand();
  }

  /**
   * Opens an http or https link in the browser, and the Mini App stays open. Answers false
   * outside Telegram, where the page's own link navigates instead.
   */
  openLink(url: string): boolean {
    if (this.#webApp === undefined) return false;
    this.#webApp.openLink(url);
    return true;
  }
}

/** A wrapper over `host`'s Telegram object, read once. */
export function connect(host: Host | undefined): TelegramWrapper {
  return new TelegramWrapper(host);
}

/** The app's wrapper, over the page's own global object. */
export const telegram = connect(
  typeof window === 'undefined' ? undefined : (window as unknown as Host)
);
