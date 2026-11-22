<script lang="ts">
  import { api, MANAGER_LABELS, toRef, type ManagerId, type Package } from '$lib/api';
  import { t } from '$lib/i18n.svelte';
  import { isInstalled, managers, op, runOp } from '$lib/state.svelte';

  let query = $state('');
  let searched = $state('');
  let pending = $state(0);
  let results = $state<Partial<Record<ManagerId, Package[]>>>({});
  let errors = $state<Partial<Record<ManagerId, string>>>({});
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
  });

  const key = (p: Package) => `${p.manager}/${p.id}`;
  const groups = $derived(
    managers()
      .map((m) => ({ m, list: results[m] ?? [], error: errors[m] }))
      .filter((g) => g.list.length || g.error),
  );
  const found = $derived(groups.reduce((n, g) => n + g.list.length, 0));

  async function search(e?: Event) {
    e?.preventDefault();
    const q = query.trim();
    if (!q) return;
    searched = q;
    results = {};
    errors = {};
    const list = managers();
    pending = list.length;
    // Show each manager's results as soon as they arrive.
    await Promise.all(
      list.map(async (m) => {
        try {
          const r = await api.search(m, q);
          if (searched === q) results[m] = r;
        } catch (err) {
          if (searched === q) errors[m] = String(err);
        } finally {
          if (searched === q) pending -= 1;
        }
      }),
    );
  }

  function install(p: Package) {
    runOp(`${t('search.install')}: ${p.name}`, () => api.runAction('install', [toRef(p)]), [p.manager]);
  }
</script>

<header class="top">
  <h1>{t('search.title')}</h1>
</header>

<form class="bar" onsubmit={search}>
  <input bind:this={input} type="search" placeholder={t('search.placeholder')} bind:value={query} />
  <button class="primary" type="submit" disabled={!query.trim()}>{t('search.button')}</button>
</form>

{#if !searched}
  <p class="muted small">{t('search.hint')}</p>
{:else if pending > 0}
  <p class="muted small"><span class="spinner"></span> {t('search.searching')}</p>
{:else if found === 0 && !groups.length}
  <p class="muted">{t('search.none', { q: searched })}</p>
{/if}

{#each groups as g (g.m)}
  <div class="group-title">
    {MANAGER_LABELS[g.m]}
    <span class="badge">{g.list.length}</span>
  </div>
  {#if g.m === 'mas' && g.list.length}
    <p class="muted small">{t('search.masNote')}</p>
  {/if}
  {#if g.error}
    <p class="notice error small">{t('common.error', { manager: MANAGER_LABELS[g.m], message: g.error })}</p>
  {/if}
  {#if g.list.length}
    <div class="list">
      {#each g.list as p (key(p))}
        <div class="row">
          <div class="main">
            <div class="name">
              {p.name}
              {#if p.classic}<span class="badge">classic</span>{/if}
            </div>
            {#if p.description}<div class="desc">{p.description}</div>
            {:else if p.name !== p.id}<div class="desc">{p.id}</div>{/if}
          </div>
          {#if p.version}<span class="version">{p.version}</span>{/if}
          {#if isInstalled(p)}
            <span class="badge ok">{t('search.installed')}</span>
          {:else}
            <button onclick={() => install(p)} disabled={op.running}>
              {p.manager === 'mas' ? t('search.openAppStore') : t('search.install')}
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/each}

<style>
  .top {
    margin-bottom: 16px;
  }

  .bar {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }

  .bar input {
    flex: 1;
    max-width: 480px;
  }
</style>
