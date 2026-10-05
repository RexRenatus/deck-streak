// SPEC-350 R4; ADR-361. The engine's languages for the app's locale. Stub: every locale is English.
import { getLocale } from '$lib/paraglide/runtime.js';

/** The languages the engine's `open` receives for `locale`, the app's own when none is named. */
export function engineLanguages(locale: string = getLocale()): string[] {
  void locale;
  return ['en'];
}
