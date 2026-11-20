<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { SvelteSet } from 'svelte/reactivity';
  import { api, MANAGER_LABELS, toRef, type PreviewEntry, type SetupPreview } from '$lib/api';
  import { t, tMessage } from '$lib/i18n.svelte';
  import { app, managersOf, op, runOp } from '$lib/state.svelte';
  import { formatDate } from '$lib/time';

  let exporting = $state(false);
  let exportMsg = $state<{ cls: string; text: string } | null>(null);
  let preview = $state<SetupPreview | null>(null);
  let importMsg = $state<{ cls: string; text: string } | null>(null);
  let reading = $state(false);
  const selected = new SvelteSet<string>();

  const key = (e: PreviewEntry) => `${e.manager}/${e.id}`;
  const missing = $derived(preview?.entries.filter((e) => e.status === 'missing') ?? []);

  async function doExport() {
    exportMsg = null;
    const today = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `suisha-${app.info?.os ?? 'setup'}-${today}.json`,
      filters: [{ name: t('setup.fileFilter'), extensions: ['json'] }],
    });
    if (!path) return;
    exporting = true;
    try {
      const n = await api.exportSetup(path);
      exportMsg = { cls: 'ok', text: t('setup.exported', { n, path }) };
    } catch (e) {
      exportMsg = { cls: 'error', text: String(e) };
    } finally {
      exporting = false;
    }
  }

  async function doOpen() {
    importMsg = null;
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t('setup.fileFilter'), extensions: ['json'] }],
    });
    if (!path || Array.isArray(path)) return;
    reading = true;
    preview = null;
    selected.clear();
    try {
      preview = await api.readSetup(path);
      for (const e of preview.entries) if (e.status === 'missing') selected.add(key(e));
      if (!missing.length) importMsg = { cls: 'ok', text: t('setup.allPresent') };
    } catch (e) {
      importMsg = { cls: 'error', text: tMessage(String(e)) };
    } finally {
      reading = false;
    }
  }

  async function install() {
    if (!preview) return;
    const refs = preview.entries.filter((e) => selected.has(key(e))).map(toRef);
    if (!refs.length) return;
    await runOp(t('setup.install', { n: refs.length }), () => api.runAction('install', refs), managersOf(refs));
    // Re-check what is still missing.
    const installedNow = new Set(
      Object.values(app.installed).flatMap((l) => (l ?? []).map((p) => `${p.manager}/${p.id}`)),
    );
    for (const e of preview.entries) {
      if (e.status === 'missing' && installedNow.has(key(e))) {
        e.status = 'installed';
        selected.delete(key(e));
      }
    }
  }

  function statusText(e: PreviewEntry) {
    switch (e.status) {
      case 'missing':
        return t('setup.statusMissing');
      case 'installed':
        return t('setup.statusInstalled');
      case 'unavailable':
        return t('setup.statusUnavailable', { manager: MANAGER_LABELS[e.manager] });
      default:
        return t('setup.statusInvalid');
    }
  }
</script>

<header class="top">
  <h1>{t('setup.title')}</h1>
</header>

<div class="cards">
  <section class="card">
    <h2>{t('setup.exportTitle')}</h2>
    <p class="muted">{t('setup.exportText')}</p>
    <div class="actions">
      <button class="primary" onclick={doExport} disabled={exporting}>
        {#if exporting}<span class="spinner"></span>{/if}
        {t('setup.exportButton')}
      </button>
    </div>
    {#if exportMsg}<p class="notice {exportMsg.cls} small">{exportMsg.text}</p>{/if}
  </section>

  <section class="card">
    <h2>{t('setup.importTitle')}</h2>
    <p class="muted">{t('setup.importText')}</p>
    <div class="actions">
      <button onclick={doOpen} disabled={reading || op.running}>
        {#if reading}<span class="spinner"></span>{/if}
        {t('setup.importButton')}
      </button>
    </div>
    {#if importMsg}<p class="notice {importMsg.cls} small">{importMsg.text}</p>{/if}
  </section>
</div>

{#if preview && preview.entries.length}
  <div class="summary">
    <p class="muted">{t('setup.fileFrom', { date: formatDate(preview.exportedAt) })}</p>
    <button class="link" onclick={() => missing.forEach((e) => selected.add(key(e)))}>{t('setup.selectMissing')}</button>
    <button class="link" onclick={() => selected.clear()}>{t('setup.selectNone')}</button>
    <button class="primary" onclick={install} disabled={!selected.size || op.running}>
      {t('setup.install', { n: selected.size })}
    </button>
  </div>
  <div class="list">
    {#each preview.entries as e (key(e))}
      <label class="row" class:dim={e.status !== 'missing'}>
        <input
          type="checkbox"
          disabled={e.status !== 'missing'}
          checked={selected.has(key(e))}
          onchange={(ev) => (ev.currentTarget.checked ? selected.add(key(e)) : selected.delete(key(e)))}
        />
        <div class="main">
          <div class="name">{e.name}</div>
          {#if e.name !== e.id}<div class="desc">{e.id}</div>{/if}
        </div>
        <span class="badge">{MANAGER_LABELS[e.manager]}</span>
        <span class="badge {e.status === 'installed' ? 'ok' : e.status === 'missing' ? 'accent' : ''}">
          {statusText(e)}
        </span>
      </label>
    {/each}
  </div>
{/if}

<style>
  .top {
    margin-bottom: 16px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 14px;
  }

  .card {
    display: grid;
    gap: 10px;
    align-content: start;
  }

  .card h2 {
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 22px 0 10px;
  }

  .summary p {
    flex: 1;
  }

  label.row {
    cursor: pointer;
  }

  .dim {
    opacity: 0.6;
    cursor: default;
  }
</style>
