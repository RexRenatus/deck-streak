<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import CourseLadder from '$lib/progress/CourseLadder.svelte';
  import type { ProgressView } from '$lib/progress/progress';
  import { m } from '$lib/paraglide/messages.js';

  // The progress screen (SPEC-077 R17): each configured course's Road to C2 as the server answers
  // it, the same figures the bot's /progress reports. The page computes none of them.
  let progress = $state<Answer<ProgressView>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.progress()).then((answer) => {
      progress = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if progress === undefined}
      <div role="status">{m.loading()}</div>
    {:else if progress.kind === 'ok'}
      <section aria-labelledby="progress-title">
        <h2 id="progress-title" class="text-sm font-medium">{m.progress_title()}</h2>
        {#each progress.value.courses as course (course.code)}
          <CourseLadder {course} />
        {:else}
          <p class="mt-2">{m.progress_none()}</p>
        {/each}
      </section>
    {:else if progress.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
