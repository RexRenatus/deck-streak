// The voice picker's rendered accessibility audit (SPEC-506 R9, A6; ADR-524).
//
// axe-core runs over the review screen while it shows a card whose clips speak Japanese, which the
// device has a local voice for, and Korean, which it has only a remote voice for, so the voice picker
// lists a local voice and the status region names the language with none. It runs in Telegram's
// light and dark themes, against the WCAG 2.2 A and AA rules, as the app's audit (a11y.spec.ts) runs
// every route. The engine's Worker and the device's speech are answered in the page, so the
// production build shows the card with no engine module and nothing reaches the network.
//
// src/lib/study/voice.test.ts holds this spec's shape, since the tdd probe resolves no Playwright
// command.
//
// Run: pnpm exec playwright test tests/voice-picker-a11y.spec.ts
import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { TELEGRAM_SDK, answerLaunches, launchFragment } from './launch-fragment';
import { THEMES } from './telegram-palettes';

// axe-core's tags for WCAG 2.0, 2.1 and 2.2 at levels A and AA, as the app's audit names them.
const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/**
 * A stand-in for Telegram's script in `scheme`, as the app's audit serves it: it sets the
 * --tg-theme-* variables from the palette, which the design tokens read, and the Mini App object the
 * wrapper reads.
 */
function standIn(scheme: string, themeParams: Readonly<Record<string, string>>): string {
  return `(() => {
    const noop = () => {};
    const inset = { top: 0, bottom: 0, left: 0, right: 0 };
    const button = { show: noop, hide: noop, onClick: noop, offClick: noop, setText: noop };
    const themeParams = ${JSON.stringify(themeParams)};
    const style = document.documentElement.style;
    for (const [key, value] of Object.entries(themeParams)) {
      style.setProperty('--tg-theme-' + key.split('_').join('-'), value);
    }
    style.setProperty('--tg-color-scheme', ${JSON.stringify(scheme)});
    window.Telegram = {
      WebApp: {
        initData: 'auth_date=1&hash=synthetic',
        version: '9.0',
        platform: 'unknown',
        colorScheme: ${JSON.stringify(scheme)},
        themeParams,
        viewportStableHeight: window.innerHeight,
        safeAreaInset: inset,
        contentSafeAreaInset: inset,
        isVersionAtLeast: () => true,
        ready: noop,
        expand: noop,
        openLink: noop,
        onEvent: noop,
        offEvent: noop,
        BackButton: button,
        MainButton: button,
        HapticFeedback: { impactOccurred: noop, notificationOccurred: noop, selectionChanged: noop }
      }
    };
  })();`;
}

/**
 * Runs in the page before its scripts. The engine's Worker answers each request in the page: one new
 * card whose question speaks Japanese and then Korean. The device's speech synthesis lists a remote
 * Japanese voice (the browser's default), a local Japanese voice and a remote Korean voice, and ends
 * each utterance it is given.
 */
function device(): void {
  const japanese = { kind: 'speech', text: '犬', language: 'ja-JP', rate: 0.5 };
  const korean = { kind: 'speech', text: '개', language: 'ko-KR', rate: 0.5 };
  const side = (text: string) => ({ text, css: '', autoplay: [japanese, korean], replay: [japanese, korean], omitted: [] });
  const card = {
    id: 1n,
    ordinal: 0,
    flag: 0,
    question: '<p>dog</p>',
    answer: '<p>犬, 개</p>',
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false
  };
  const answers: Record<string, unknown> = {
    open: { existed: true },
    sync: { status: 'held', required: null },
    card: { counts: { new: 1, learning: 0, review: 0 }, card },
    faces: { question: side('<p>dog</p>'), answer: side('<p>犬, 개</p>'), wanted: [] },
    'undo-offer': { offer: null, why: 'none' }
  };
  class Engine extends EventTarget {
    postMessage(message: { id: number; op: string }): void {
      const value = answers[message.op] ?? null;
      setTimeout(() => this.dispatchEvent(new MessageEvent('message', { data: { id: message.id, ok: true, value } })), 0);
    }

    terminate(): void {}
  }
  Object.defineProperty(window, 'Worker', { configurable: true, writable: true, value: Engine });

  const voices = [
    { voiceURI: 'remote-ja', name: 'Remote Japanese', lang: 'ja-JP', localService: false, default: true },
    { voiceURI: 'local-ja', name: 'Local Japanese', lang: 'ja-JP', localService: true, default: false },
    { voiceURI: 'remote-ko', name: 'Remote Korean', lang: 'ko-KR', localService: false, default: false }
  ];
  class Synthesis extends EventTarget {
    getVoices() {
      return voices;
    }

    speak(utterance: { onend: ((event: Event) => void) | null }): void {
      setTimeout(() => utterance.onend?.(new Event('end')), 0);
    }

    cancel(): void {}
  }
  class Utterance {
    lang = '';
    rate = 1;
    voice: unknown = null;
    onend: ((event: Event) => void) | null = null;
    onerror: ((event: Event) => void) | null = null;

    constructor(readonly text: string) {}
  }
  Object.defineProperty(window, 'speechSynthesis', { configurable: true, value: new Synthesis() });
  Object.defineProperty(window, 'SpeechSynthesisUtterance', { configurable: true, writable: true, value: Utterance });
}

for (const [scheme, themeParams] of Object.entries(THEMES)) {
  test.describe(`${scheme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.route(`${TELEGRAM_SDK}*`, (intercepted) =>
        intercepted.fulfill({ contentType: 'text/javascript', body: standIn(scheme, themeParams) })
      );
      await page.addInitScript((fragment) => {
        if (window === window.top && location.hash === '') history.replaceState(history.state, '', fragment);
      }, launchFragment('auth_date=1&hash=synthetic'));
      await page.addInitScript(device);
      // the API, answered in place, as the app's audit answers it: a session, the study day, the wallet
      await answerLaunches(page, 204);
      await page.route('**/api/session', (intercepted) => intercepted.fulfill({ status: 200 }));
      await page.route('**/api/me', (intercepted) => intercepted.fulfill({ json: { study_day: '2001-02-03' } }));
      await page.route('**/api/wallet', (intercepted) =>
        intercepted.fulfill({
          json: {
            study_day: '2001-02-03',
            balance: 103,
            loss_cap: 30,
            loss_cap_left: 25,
            movements: [
              { id: 4, study_day: '2001-02-03', source: 'fine', amount: -5 },
              { id: 3, study_day: '2001-02-03', source: 'mint', amount: 8 }
            ],
            next: 3
          }
        })
      );
      await page.emulateMedia({ colorScheme: scheme as 'light' | 'dark' });
    });

    test('the voice picker and the no-voice line have no WCAG 2.2 AA violation axe can find', async ({ page }) => {
      await page.goto('/study/review');
      // the stand-in ran: the root carries the palette's scheme it sets
      await expect
        .poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--tg-color-scheme')))
        .toBe(scheme);
      // the picker lists the local Japanese voice alone, and the line names Korean
      const japanese = page.getByRole('combobox', { name: 'Voice for ja-JP' });
      await expect(japanese.getByRole('option')).toHaveText(['Default voice', 'Local Japanese']);
      await expect(page.getByRole('combobox', { name: 'Voice for ko-KR' }).getByRole('option')).toHaveText([
        'Default voice'
      ]);
      const review = page.getByRole('region', { name: 'Review', exact: true });
      // the no-voice line, by its text: the update offer below it is a status region of the review too
      const line = review.getByRole('status').filter({ hasText: 'No voice on this device' });
      await expect(line).toHaveText('No voice on this device for ko-KR.');
      // the card frame is sandboxed with no script, so the legacy mode audits the screen around it,
      // as the study suite does
      const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).setLegacyMode().analyze();
      expect(results.violations).toEqual([]);
    });
  });
}
