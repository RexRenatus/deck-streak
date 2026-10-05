// SPEC-350 R4; ADR-361. The engine's languages for the app's locale. The engine's own translations
// name Chinese by region, where the app names it by script; every other locale is the same tag.
import { getLocale, type Locale } from '$lib/paraglide/runtime.js';

/** The engine's language for each of the app's locales. */
const ENGINE: Record<Locale, string> = {
  en: 'en',
  es: 'es',
  fr: 'fr',
  ja: 'ja',
  ko: 'ko',
  'zh-Hans': 'zh-CN',
  'zh-Hant': 'zh-TW'
};

/** The languages the engine's `open` receives for `locale`, the app's own when none is named. */
export function engineLanguages(locale: Locale = getLocale()): string[] {
  return [ENGINE[locale]];
}
