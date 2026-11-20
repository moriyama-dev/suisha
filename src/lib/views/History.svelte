<script lang="ts">
  import { onMount } from 'svelte';
  import { api, MANAGER_LABELS, type HistoryEntry } from '$lib/api';
  import { t, tMessage, type Key } from '$lib/i18n.svelte';
  import { op } from '$lib/state.svelte';
  import { formatTime } from '$lib/time';

  let entries = $state<HistoryEntry[]>([]);
  let loaded = $state(false);

  async function load() {
    entries = await api.getHistory();
    loaded = true;
  }

  onMount(load);

  $effect(() => {
    // Reload after each change finishes.
    if (!op.running && op.result) load();
  });

  function outcome(e: HistoryEntry) {
    if (e.success) return { cls: 'ok', text: t('history.ok') };
    if (e.message === 'cancelled') return { cls: '', text: t('history.cancelled') };
    if (e.message === 'history.adminSkipped' || e.message === 'auto.noHelpers')
      return { cls: '', text: t('history.skipped') };
    return { cls: 'error', text: t('history.failed') };
  }
</script>

<header class="top">
  <h1>{t('history.title')}</h1>
</header>

{#if loaded && !entries.length}
  <p class="muted">{t('history.empty')}</p>
{:else}
  <div class="list">
    {#each entries as e, i (i)}
      {@const o = outcome(e)}
      <div class="row entry">
        <div class="when">
          <div class="small">{formatTime(e.time)}</div>
          <span class="badge {e.trigger === 'scheduled' ? 'accent' : ''}">
            {e.trigger === 'scheduled' ? t('history.scheduled') : t('history.manual')}
          </span>
        </div>
        <div class="main">
          <div>
            <strong>{t(`action.${e.action}` as Key)}</strong>
            <span class="muted"> · {e.managers.map((m) => MANAGER_LABELS[m]).join(', ')}</span>
          </div>
          <div class="desc">{e.packages.length ? e.packages.join(', ') : t('history.all')}</div>
          {#if !e.success && e.message && e.message !== 'cancelled'}
            <details>
              <summary class="small muted">{t('op.showLog')}</summary>
              <pre>{tMessage(e.message)}</pre>
            </details>
          {/if}
        </div>
        <span class="badge {o.cls}">{o.text}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .top {
    margin-bottom: 16px;
  }

  .entry {
    align-items: flex-start;
  }

  .when {
    width: 150px;
    flex: none;
    display: grid;
    gap: 3px;
    justify-items: start;
  }

  pre {
    margin: 6px 0 0;
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-all;
    background: var(--surface-2);
    padding: 8px 10px;
    border-radius: 6px;
  }
</style>
