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

/** Telegram's Mini App object. */
export type WebApp = Readonly<Record<string, unknown>>;

/** What the wrapper needs of the page's global object. */
export interface Host {
  readonly Telegram?: { readonly WebApp?: WebApp };
}

/** No inset on any edge. */
export const NO_INSETS: Insets = Object.freeze({ top: 0, bottom: 0, left: 0, right: 0 });

/** The one wrapper around Telegram's Mini App object. */
export class TelegramWrapper {
  readonly inside: boolean = false;
  readonly platform: string = 'unknown';
  readonly version: string = '';
  readonly startParam: string | null = null;
  readonly launchData: string | null = null;
  colorScheme = $state<ColorScheme>('light');
  themeParams = $state<ThemeParams>({});
  viewportStableHeight = $state<number | null>(null);
  safeAreaInset = $state<Insets>(NO_INSETS);
  contentSafeAreaInset = $state<Insets>(NO_INSETS);

  constructor(host: Host | undefined) {
    void host;
  }

  ready(): void {}

  openLink(url: string): boolean {
    void url;
    return false;
  }
}

/** A wrapper over `host`. */
export function connect(host: Host | undefined): TelegramWrapper {
  return new TelegramWrapper(host);
}

/** The app's wrapper. */
export const telegram = connect(
  typeof window === 'undefined' ? undefined : (window as unknown as Host)
);
