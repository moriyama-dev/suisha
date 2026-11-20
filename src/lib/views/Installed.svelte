<script lang="ts">
  import { api, MANAGER_LABELS, toRef, type ManagerId, type Package } from '$lib/api';
  import { t } from '$lib/i18n.svelte';
  import { app, loadInstalled, managers, op, runOp } from '$lib/state.svelte';

  let filter = $state('');
  let only = $state<ManagerId | 'all'>('all');
  let confirming = $state('');

  const key = (p: Package) => `${p.manager}/${p.scope}/${p.id}`;
  const groups = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    return managers()
      .filter((m) => only === 'all' || only === m)
      .map((m) => ({
        m,
        error: app.installedErrors[m],
        list: (app.installed[m] ?? []).filter(
          (p) =>
            !q ||
            p.name.toLowerCase().includes(q) ||
            p.id.toLowerCase().includes(q) ||
            p.description.toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.list.length || g.error);
  });
  const total = $derived(groups.reduce((n, g) => n + g.list.length, 0));

  function remove(p: Package) {
    if (confirming !== key(p)) {
      confirming = key(p);
      return;
    }
    confirming = '';
    runOp(`${t('installed.remove')}: ${p.name}`, () => api.runAction('remove', [toRef(p)]), [p.manager]);
  }
</script>

<header class="top">
  <h1>{t('installed.title')}</h1>
</header>

<div class="tools">
  <input type="search" placeholder={t('installed.filter')} bind:value={filter} />
  <select bind:value={only} aria-label={t('nav.installed')}>
    <option value="all">{t('installed.allManagers')}</option>
    {#each managers() as m (m)}
      <option value={m}>{MANAGER_LABELS[m]}</option>
    {/each}
  </select>
  <span class="muted small">{t('installed.count', { n: total })}</span>
  <span class="spacer"></span>
  <button onclick={() => loadInstalled()} disabled={app.installedLoading}>
    {#if app.installedLoading}<span class="spinner"></span>{/if}
    {t('updates.check')}
  </button>
</div>
<p class="muted small">{t('installed.note')}</p>

{#if app.installedLoading && total === 0}
  <p class="muted pad"><span class="spinner"></span> {t('common.loading')}</p>
{:else if total === 0 && groups.every((g) => !g.error)}
  <p class="muted pad">{t('installed.empty')}</p>
{/if}

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
          <div class="main">
            <div class="name">
              {p.name}
              {#if p.scope === 'user'}<span class="badge">user</span>{/if}
              {#if p.classic}<span class="badge">classic</span>{/if}
            </div>
            {#if p.description}<div class="desc">{p.description}</div>
            {:else if p.name !== p.id}<div class="desc">{p.id}</div>{/if}
          </div>
          <span class="version">{p.version}</span>
          <button
            class="danger"
            class:confirm={confirming === key(p)}
            onclick={() => remove(p)}
            onblur={() => confirming === key(p) && (confirming = '')}
            disabled={op.running}
          >
            {confirming === key(p) ? t('installed.confirmRemove') : t('installed.remove')}
          </button>
        </div>
      {/each}
    </div>
  {/if}
{/each}

<style>
  .top {
    margin-bottom: 16px;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }

  .tools input {
    flex: 1;
    max-width: 360px;
  }

  .spacer {
    flex: 1;
  }

  .pad {
    padding: 24px 0;
  }

</style>
