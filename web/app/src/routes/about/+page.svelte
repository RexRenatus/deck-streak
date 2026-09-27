<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { telegram } from '$lib/telegram.svelte';

  // The privacy policy, and the source code: the AGPL's offer to everyone who uses DeckStreak over
  // a network (ADR-018).
  const PRIVACY_POLICY = 'https://github.com/RexRenatus/deck-streak/blob/main/PRIVACY.md';
  const SOURCE_CODE = 'https://github.com/RexRenatus/deck-streak';

  /**
   * Inside Telegram a link opens in the browser through the wrapper, and the Mini App stays open;
   * elsewhere the link navigates as a link does.
   */
  function outward(url: string) {
    return (event: MouseEvent) => {
      if (telegram.openLink(url)) event.preventDefault();
    };
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.about_title()}</h1>
  <p class="mt-3">{m.about_license()}</p>

  <ul class="mt-6 space-y-2">
    <li><a href={PRIVACY_POLICY} onclick={outward(PRIVACY_POLICY)}>{m.privacy_policy()}</a></li>
    <li><a href={SOURCE_CODE} onclick={outward(SOURCE_CODE)}>{m.source_code()}</a></li>
  </ul>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
