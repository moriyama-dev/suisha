<script lang="ts">
  import { onMount } from 'svelte';
  import { api, MANAGER_LABELS, type HistoryEntry } from '$lib/api';
  import { t, tMessage, type Key } from '$lib/i18n.svelte';
  import { app, op } from '$lib/state.svelte';
  import { formatTime } from '$lib/time';

  let entries = $state<HistoryEntry[]>([]);
  let loaded = $state(false);
  // Long histories are shown a page at a time.
  const PAGE = 200;
  let shown = $state(PAGE);
  let error = $state('');

  // null keeps every entry, 0 keeps none.
  const PRESETS: (number | null)[] = [0, 3, 30, 100, 400, null];
  const MAX_DAYS = 3650;
  const key = (d: number | null) => (d === null ? 'forever' : String(d));

  const keep = $derived(app.settings ? app.settings.historyKeepDays : null);
  let customOpen = $state(false);
  let customDays = $state(30);
  // A change that would delete entries waits here until it is confirmed.
  let pending = $state<{ days: number | null; lost: number } | null>(null);

  const isCustom = $derived(customOpen || !PRESETS.includes(keep));
  const customValid = $derived(Number.isInteger(customDays) && customDays >= 1 && customDays <= MAX_DAYS);

  // A saved number of days that is not on the list stays editable in the field.
  $effect(() => {
    if (!customOpen && keep !== null && !PRESETS.includes(keep)) customDays = keep;
  });

  function keepLabel(days: number | null) {
    if (days === null) return t('history.keepForever');
    if (days === 0) return t('history.keepNone');
    return t('history.keepDays', { n: days });
  }

  function lostWith(days: number | null) {
    if (days === null) return 0;
    if (days === 0) return entries.length;
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    return entries.filter((e) => Date.parse(e.time) < cutoff).length;
  }

  function choose(value: string) {
    pending = null;
    if (value === 'custom') {
      customOpen = true;
      return;
    }
    customOpen = false;
    request(value === 'forever' ? null : Number(value));
  }

  function request(days: number | null) {
    const lost = lostWith(days);
    if (lost > 0) pending = { days, lost };
    else setKeep(days);
  }

  async function setKeep(days: number | null) {
    error = '';
    pending = null;
    try {
      app.settings = await api.setHistoryKeepDays(days);
      customOpen = false;
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function cancelPending() {
    pending = null;
    customOpen = false;
  }

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
{#if app.settings}
  <!-- Kept on its own row: the top right corner belongs to the version button. -->
  <div class="keep">
    <label>
      <span class="muted small">{t('history.keep')}</span>
      <select
        value={pending ? key(pending.days) : isCustom ? 'custom' : key(keep)}
        onchange={(e) => choose(e.currentTarget.value)}
      >
        {#each PRESETS as d (key(d))}
          <option value={key(d)}>{keepLabel(d)}</option>
        {/each}
        <option value="custom">{t('history.keepCustom')}</option>
      </select>
    </label>
    {#if isCustom}
      <label>
        <span class="muted small">{t('history.keepCustomDays', { max: MAX_DAYS })}</span>
        <input type="number" min="1" max={MAX_DAYS} step="1" bind:value={customDays} />
      </label>
      <button
        class="primary"
        disabled={!customValid || !!pending || customDays === keep}
        onclick={() => request(customDays)}
      >
        {t('common.save')}
      </button>
      {#if customOpen}
        <button disabled={!!pending} onclick={() => (customOpen = false)}>{t('common.cancel')}</button>
      {/if}
    {:else if !pending}
      {#if keep === 0}
        <span class="muted small">{t('history.keepNoneNote')}</span>
      {:else if keep !== null}
        <span class="muted small">{t('history.keepNote')}</span>
      {/if}
    {/if}
  </div>
  {#if pending}
    <div class="notice warn confirm">
      <span>{t('history.keepConfirm', { n: pending.lost })}</span>
      <button class="danger" onclick={() => pending && setKeep(pending.days)}>
        {t('history.keepConfirmButton')}
      </button>
      <button onclick={cancelPending}>{t('common.cancel')}</button>
    </div>
  {/if}
{/if}
{#if error}<p class="notice error small note">{error}</p>{/if}

{#if loaded && !entries.length}
  <p class="muted">{t('history.empty')}</p>
{:else}
  <div class="list">
    {#each entries.slice(0, shown) as e, i (i)}
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
  {#if entries.length > shown}
    <p class="more"><button onclick={() => (shown += PAGE)}>{t('history.more')}</button></p>
  {/if}
{/if}

<style>
  .top {
    margin-bottom: 12px;
  }

  .keep {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    margin-bottom: 14px;
  }

  .keep label {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .keep input {
    width: 90px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface);
  }

  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    margin-bottom: 14px;
  }

  .confirm span {
    flex: 1;
    min-width: 200px;
  }

  .note {
    margin-bottom: 12px;
  }

  .more {
    margin-top: 12px;
    text-align: center;
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
