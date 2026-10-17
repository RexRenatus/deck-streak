import type { ClientInit } from '@sveltejs/kit';
import { getLocale, getTextDirection } from '$lib/paraglide/runtime';
import { admitLaunch } from '$lib/telegram-launch';

// The fallback page is rendered once, at build, in the base locale. When the app starts, the
// document takes the visitor's locale, so `:lang()` styles (the CJK font stacks and line
// breaking) and assistive technology see the language the page is in.
export const init: ClientInit = async () => {
  document.documentElement.lang = getLocale();
  document.documentElement.dir = getTextDirection();
  // SPEC-400 R2, R3: the router's first navigation waits for this hook, so the launch is read while
  // the fragment Telegram opened the page with is whole, and its script has loaded before any
  // component asks for it.
  await admitLaunch(window);
};
