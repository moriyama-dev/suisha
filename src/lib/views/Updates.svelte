<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import { api, MANAGER_LABELS, toRef, type Package } from '$lib/api';
  import AutoUpdate from '$lib/components/AutoUpdate.svelte';
  import SystemNotices from '$lib/components/SystemNotices.svelte';
  import { t, type Key } from '$lib/i18n.svelte';
  import { app, loadSystem, loadUpdates, managers, managersOf, op, runOp, updateCount } from '$lib/state.svelte';
  import { formatTime } from '$lib/time';

  const selected = new SvelteSet<string>();
  const key = (p: Package) => `${p.manager}/${p.scope}/${p.id}`;
  const count = $derived(updateCount());
  const groups = $derived(
    managers()
      .map((m) => ({
        m,
        list: (app.updates[m] ?? []).filter((p) => !p.held),
        error: app.updatesErrors[m],
      }))
      .filter((g) => g.list.length || g.error),
  );
  // Updates apt will not install yet (phased or kept back).
  const held = $derived(managers().flatMap((m) => (app.updates[m] ?? []).filter((p) => p.held)));
  const allUpdates = $derived(groups.flatMap((g) => g.list));

  async function check() {
    if (op.running) return;
    // `brew update` downloads fresh formula data; the others ask live.
    if (managers().includes('brew')) {
      await runOp(t('updates.check'), () => api.refresh(), []);
      op.result = null;
    } else {
      await Promise.all([loadUpdates(), loadSystem()]);
    }
  }

  function updateAll() {
    selected.clear();
    runOp(t('updates.updateAll'), () => api.updateAll(managers()), []);
  }

  function update(list: Package[]) {
    const refs = list.map(toRef);
    for (const p of list) selected.delete(key(p));
    runOp(t('updates.update'), () => api.runAction('update', refs), managersOf(refs));
  }

  function toggle(p: Package, on: boolean) {
    if (on) selected.add(key(p));
    else selected.delete(key(p));
  }
</script>

<header class="top">
  <h1>{t('updates.title')}</h1>
</header>

<AutoUpdate />

<section class="updates">
  <div class="section-head">
    <h2>{t('updates.listTitle')}</h2>
    {#if app.updatesCheckedAt}
      <span class="muted small">
        {t('updates.checkedAt', { time: formatTime(new Date(app.updatesCheckedAt).toISOString()) })}
      </span>
    {/if}
    <span class="spacer"></span>
    <button onclick={check} disabled={op.running || app.updatesLoading}>
      {#if app.updatesLoading}<span class="spinner"></span>{/if}
      {t('updates.check')}
    </button>
  </div>

  <SystemNotices />

  <div class="summary">
    <p>
      {#if app.updatesLoading && !count}
        <span class="muted">{t('updates.checking')}</span>
      {:else if count === 0}
        <strong>{t('updates.none')}</strong>
      {:else}
        <strong>{count === 1 ? t('updates.countOne') : t('updates.count', { n: count })}</strong>
      {/if}
    </p>
    {#if selected.size}
      <button onclick={() => update(allUpdates.filter((p) => selected.has(key(p))))} disabled={op.running}>
        {t('updates.updateSelected', { n: selected.size })}
      </button>
    {/if}
    <button class="primary" onclick={updateAll} disabled={op.running || managers().length === 0}>
      {t('updates.updateAll')}
    </button>
  </div>

  {#each groups as g (g.m)}
    <div class="group-title">
      {MANAGER_LABELS[g.m]}
      <span class="badge">{g.list.length}</span>
    </div>
    {#if g.error}
      <p class="notice error small">{t('common.error', { manager: MANAGER_LABELS[g.m], message: g.error })}</p>
    {/if}
    {#if g.list.length}
      <div class="list">
        {#each g.list as p (key(p))}
          <div class="row">
            <input
              type="checkbox"
              aria-label={p.name}
              checked={selected.has(key(p))}
              onchange={(e) => toggle(p, e.currentTarget.checked)}
            />
            <div class="main">
              <div class="name">{p.name}</div>
              {#if p.name !== p.id}<div class="desc">{p.id}</div>{/if}
            </div>
            <span class="version">
              {#if p.version}{`${p.version} → `}{/if}<strong>{p.newVersion}</strong>
            </span>
            <button onclick={() => update([p])} disabled={op.running}>{t('updates.update')}</button>
          </div>
        {/each}
      </div>
    {/if}
  {/each}

  {#if held.length}
    <details class="held">
      <summary>
        <strong>{t('updates.heldTitle', { n: held.length })}</strong>
      </summary>
      <p class="muted small">{t('updates.heldText')}</p>
      <div class="list">
        {#each held as p (key(p))}
          <div class="row">
            <div class="main">
              <div class="name">{p.name}</div>
            </div>
            <span class="version">{#if p.version}{`${p.version} → `}{/if}{p.newVersion}</span>
            <span class="badge">{t(`updates.held.${p.held}` as Key)}</span>
          </div>
        {/each}
      </div>
    </details>
  {/if}

  {#if managers().includes('apt')}
    <p class="muted small note">{t('updates.aptNote')}</p>
  {/if}
</section>

<style>
  .top {
    margin-bottom: 18px;
  }

  .updates {
    border-top: 1px solid var(--border);
    padding-top: 22px;
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .section-head h2 {
    margin: 0;
    font-size: 16px;
  }

  .spacer {
    flex: 1;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 4px;
  }

  .summary p {
    flex: 1;
  }

  .held {
    margin-top: 22px;
  }

  .held summary {
    cursor: pointer;
    margin-bottom: 8px;
  }

  .held p {
    margin-bottom: 10px;
  }

  .held .row {
    opacity: 0.75;
  }

  .note {
    margin-top: 16px;
  }
</style>
