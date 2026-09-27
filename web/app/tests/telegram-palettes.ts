// Telegram's two default palettes, as the accessibility pack's template audit writes them
// (.packs/skills/packs/accessibility/templates/a11y.spec.ts). The Mini App paints with the
// --tg-theme-* variables Telegram sets from these parameters, which exist only inside Telegram.
//
// One module holds them so that the rendered audit (a11y.spec.ts, which paints each palette) and
// the contrast proof (src/lib/design/tokens.test.ts, which resolves every declared pair in each)
// can never judge different colours (SPEC-028 R5, R10). It imports nothing, so both Playwright and
// Vitest load it.
export const THEMES = {
  light: {
    bg_color: '#ffffff',
    text_color: '#000000',
    hint_color: '#707579',
    link_color: '#3390ec',
    button_color: '#3390ec',
    button_text_color: '#ffffff',
    secondary_bg_color: '#f4f4f5'
  },
  dark: {
    bg_color: '#212121',
    text_color: '#ffffff',
    hint_color: '#aaaaaa',
    link_color: '#8774e1',
    button_color: '#8774e1',
    button_text_color: '#ffffff',
    secondary_bg_color: '#181818'
  }
} as const;

/** A colour scheme Telegram paints a Mini App in. */
export type Scheme = keyof typeof THEMES;

/** A theme parameter both palettes define. */
export type ThemeParam = keyof (typeof THEMES)['light'];
