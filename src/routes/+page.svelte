<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import OpPanel from '$lib/components/OpPanel.svelte';
  import SelfUpdate from '$lib/components/SelfUpdate.svelte';
  import Wheel from '$lib/components/Wheel.svelte';
  import { openRepo, reportProblem } from '$lib/feedback';
  import { i18n, setLanguage, t, type Key, type LangChoice } from '$lib/i18n.svelte';
  import { scheduleText } from '$lib/schedule';
  import {
    app,
    loadInfo,
    loadInstalled,
    loadSettings,
    loadSystem,
    loadUpdates,
    managers,
    needsAdminAccess,
    op,
    updateCount,
  } from '$lib/state.svelte';
  import History from '$lib/views/History.svelte';
  import Installed from '$lib/views/Installed.svelte';
  import Search from '$lib/views/Search.svelte';
  import Setup from '$lib/views/Setup.svelte';
  import Updates from '$lib/views/Updates.svelte';

  type View = 'updates' | 'installed' | 'search' | 'setup' | 'history';
  const views: View[] = ['updates', 'installed', 'search', 'setup', 'history'];
  const fromHash = (): View => {
    const h = location.hash.slice(1) as View;
    return views.includes(h) ? h : 'updates';
  };
  let view = $state<View>(fromHash());
  let ready = $state(false);

  function go(v: View) {
    view = v;
    history.replaceState(null, '', `#${v}`);
  }

  onMount(async () => {
    document.documentElement.lang = i18n.lang;
    window.addEventListener('hashchange', () => (view = fromHash()));
    await Promise.all([loadInfo(), loadSettings()]);
    ready = true;
    await Promise.all([loadUpdates(), loadInstalled(), loadSystem()]);
  });

  // While the window is open, look for new updates every 30 minutes, and
  // when the window comes back to the front after 10 minutes or more.
  // Reading the lists is cheap; nothing is installed without the user.
  $effect(() => {
    const recheck = (minAge: number) => {
      if (ready && !op.running && !app.updatesLoading && Date.now() - app.updatesCheckedAt >= minAge) {
        loadUpdates();
        loadSystem();
      }
    };
    const timer = setInterval(() => recheck(0), 30 * 60 * 1000);
    const onFocus = () => recheck(10 * 60 * 1000);
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(timer);
      window.removeEventListener('focus', onFocus);
    };
  });

  $effect(() => {
    // Background notifications use the same language as the window.
    api.setLanguage(i18n.lang).catch(() => {});
  });

  const count = $derived(updateCount());
  const auto = $derived(app.settings?.autoUpdate);
  // Automatic updates are on, but apt and snap still need the password registered.
  const autoNeedsPassword = $derived(
    !!auto?.enabled && needsAdminAccess() && !!app.info?.helpers && !app.info?.autoUpdateAllowed,
  );
</script>

<div class="shell">
  <nav>
    <div class="brand">
      <Wheel size={30} spinning={op.running} />
      <span>Suisha</span>
    </div>
    <ul>
      {#each views as v (v)}
        <li>
          <button class:active={view === v} onclick={() => go(v)} aria-current={view === v ? 'page' : undefined}>
            <span>{t(`nav.${v}` as Key)}</span>
            {#if v === 'updates' && count > 0}<span class="badge accent">{count}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if auto}
      <button class="auto" class:on={auto.enabled} class:warn={autoNeedsPassword} onclick={() => go('updates')}>
        <Wheel size={20} spinning={auto.enabled} />
        <span class="auto-text">
          <span class="small">{t('auto.sidebar')}</span>
          {#if autoNeedsPassword}
            <span class="small strong">{t('auto.adminRegister')}</span>
          {:else if auto.enabled}
            <span class="small muted">{t('auto.on')} · {scheduleText(auto)}</span>
          {:else}
            <span class="small muted">{t('auto.off')}</span>
          {/if}
        </span>
      </button>
    {/if}
    <div class="foot">
      <button class="link icon" onclick={() => reportProblem(app.info)}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
          <ellipse cx="8" cy="9.5" rx="3.5" ry="4.5" />
          <path d="M8 5v9M4.5 9.5H2M14 9.5h-2.5M5 6.5 3 5M11 6.5 13 5M5 12.5 3 14M11 12.5 13 14M6 5.2 5.2 3.5M10 5.2l.8-1.7" />
        </svg>
        {t('nav.report')}
      </button>
      <button class="link icon" onclick={openRepo}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="currentColor">
          <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
        </svg>
        {t('nav.github')}
      </button>
      <label class="lang">
        <span class="muted small">{t('nav.language')}</span>
        <select value={i18n.choice} onchange={(e) => setLanguage(e.currentTarget.value as LangChoice)}>
          <option value="auto">{t('nav.languageAuto')}</option>
          <option value="en">English</option>
          <option value="ja">日本語</option>
        </select>
      </label>
      {#if app.info}<span class="muted small">v{app.info.version}</span>{/if}
    </div>
  </nav>

  <div class="content">
    {#if ready}
      <div class="topbar"><SelfUpdate /></div>
    {/if}
    <main>
      {#if !ready}
        <p class="muted"><span class="spinner"></span> {t('common.loading')}</p>
      {:else if app.infoError}
        <p class="notice error">{app.infoError}</p>
      {:else}
        {#if managers().length === 0}
          <p class="notice warn">{t('common.noManagers')}</p>
        {/if}
        {#if view === 'updates'}<Updates />
        {:else if view === 'installed'}<Installed />
        {:else if view === 'search'}<Search />
        {:else if view === 'setup'}<Setup />
        {:else}<History />{/if}
      {/if}
    </main>
    <OpPanel />
  </div>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
  }

  nav {
    width: 200px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 16px 10px 14px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    font-weight: 700;
    font-size: 17px;
    padding: 0 8px 16px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  ul button {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: none;
    background: none;
    padding: 7px 10px;
    text-align: left;
  }

  ul button.active {
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 600;
  }

  .auto {
    margin-top: auto;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    padding: 9px 10px;
    border-radius: 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    white-space: normal;
  }

  .auto.on {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
  }

  .auto.warn {
    background: var(--warn-soft);
    border-color: color-mix(in srgb, var(--warn) 45%, transparent);
  }

  .auto.warn .strong {
    color: var(--warn);
    font-weight: 600;
  }

  .auto-text {
    display: grid;
    line-height: 1.35;
  }

  .foot {
    display: grid;
    gap: 8px;
    justify-items: start;
    padding: 0 10px;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .lang {
    display: grid;
    gap: 2px;
  }

  .content {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .topbar {
    position: absolute;
    top: 18px;
    right: 28px;
    z-index: 5;
  }

  main {
    flex: 1;
    overflow: auto;
    padding: 22px 28px 32px;
  }

  main > :global(*) {
    max-width: 920px;
  }
</style>
