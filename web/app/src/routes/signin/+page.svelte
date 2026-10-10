<script lang="ts">
  import { signIn } from '$lib/passkeys';

  // SPEC-385 R8: a red-first stub of the sign-in page. It offers a passkey everywhere, posts a
  // refused finish a second time, and stays after a sign-in.
  let working = $state(false);

  async function start(): Promise<void> {
    working = true;
    const answer = await signIn();
    if (answer.kind !== 'ok') await signIn();
    working = false;
  }
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">Sign in</h1>
  <button
    type="button"
    class="mt-6 inline-flex min-h-11 items-center rounded-md border px-4"
    disabled={working}
    onclick={start}
  >
    Sign in with a passkey
  </button>
</main>
