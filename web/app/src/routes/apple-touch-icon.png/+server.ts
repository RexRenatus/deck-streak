import { icon } from '$lib/icon';

// SPEC-350 R17, A28; ADR-361 D16. The 180 pixel touch icon `app.html` links, built from text when
// the site is built: the build prerenders this endpoint into the file the site serves at this path.
export const prerender = true;

export const GET = (): Promise<Response> => icon(180);
