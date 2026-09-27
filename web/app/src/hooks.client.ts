import type { ClientInit } from '@sveltejs/kit';
import { getLocale, getTextDirection } from '$lib/paraglide/runtime';

// The fallback page is rendered once, at build, in the base locale. When the app starts, the
// document takes the visitor's locale, so `:lang()` styles (the CJK font stacks and line
// breaking) and assistive technology see the language the page is in.
export const init: ClientInit = () => {
  document.documentElement.lang = getLocale();
  document.documentElement.dir = getTextDirection();
};
