<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import DarkFields from '$lib/insights/DarkFields.svelte';
  import InstrumentSection from '$lib/insights/InstrumentSection.svelte';
  import {
    DARK_FIELDS_ID,
    parseDarkFields,
    type Envelope,
    type Listing
  } from '$lib/insights/insights';

  // The insights screen (SPEC-094 R19): each instrument's latest stored report, as the server
  // answers it. Charts render here, on the client (ADR-085); the server sends numbers only.
  let listed = $state<Answer<Listing[]>>();
  let reports = $state<Record<string, Answer<Envelope | null>>>({});

  $effect(() => {
    void untrack(() => api.insights()).then(async (answer) => {
      listed = answer;
      if (answer.kind !== 'ok') return;
      for (const item of answer.value) {
        reports[item.id] = await api.insight(item.id);
      }
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.insights_title()}</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if listed === undefined}
      <div role="status">{m.loading()}</div>
    {:else if listed.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else if listed.kind === 'unavailable'}
      <div role="alert">{m.server_unavailable()}</div>
    {:else if listed.value.length === 0}
      <p>{m.insights_none()}</p>
    {:else}
      {#each listed.value as item (item.id)}
        {@const got = reports[item.id]}
        {#if got === undefined}
          <div role="status">{m.loading()}</div>
        {:else if got.kind !== 'ok'}
          <div role="alert">{got.kind === 'reopen' ? m.reopen_from_telegram() : m.server_unavailable()}</div>
        {:else if got.value === null}
          <p>{m.insights_no_report()}</p>
        {:else if item.id === DARK_FIELDS_ID}
          {@const view = parseDarkFields(got.value.report)}
          <InstrumentSection title={m.dark_fields_title()} envelope={got.value}>
            {#if view !== null}
              <DarkFields report={view} />
            {:else}
              <div role="alert">{m.server_unavailable()}</div>
            {/if}
          </InstrumentSection>
        {/if}
      {/each}
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
