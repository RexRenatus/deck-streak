<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer, type Me } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';

  // Today shows the study day the server answers (SPEC-028 R8). The day turns over at 04:00 on the
  // server's calendar, so the page never works one out from the phone's clock: it only writes the
  // server's date out in the reader's language.
  let today = $state<Answer<Me>>();

  // Ask once the screen is on the page. The call is untracked, so no state it reads can make the
  // effect ask again; a server render skips effects altogether.
  $effect(() => {
    void untrack(() => api.me()).then((answer) => {
      today = answer;
    });
  });

  /** An ISO date written out in full, as the calendar date it names, in the page's language. */
  function spelled(isoDate: string): string {
    const [year, month, day] = isoDate.split('-').map(Number);
    return new Intl.DateTimeFormat(getLocale(), { dateStyle: 'full', timeZone: 'UTC' }).format(
      Date.UTC(year, month - 1, day)
    );
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>
  <p class="mt-3 text-lg">{m.tagline()}</p>

  <section class="mt-8 rounded-lg bg-card p-4 text-card-foreground" aria-labelledby="study-day">
    <h2 id="study-day" class="text-sm font-medium">{m.study_day()}</h2>
    {#if today === undefined}
      <div class="mt-1" role="status">{m.loading()}</div>
    {:else if today.kind === 'ok'}
      <time class="mt-1 block text-xl font-semibold" datetime={today.value.studyDay}>
        {spelled(today.value.studyDay)}
      </time>
    {:else if today.kind === 'reopen'}
      <div class="mt-1" role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div class="mt-1" role="alert">{m.server_unavailable()}</div>
    {/if}
  </section>

  <nav class="mt-8 flex flex-wrap gap-x-6 gap-y-2">
    <a href="/about">{m.about()}</a>
    <a href="/sign-in-methods">{m.methods_title()}</a>
  </nav>
</main>
