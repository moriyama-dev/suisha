<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api, type SelfUpdate } from '$lib/api';
  import { t, tMessage } from '$lib/i18n.svelte';
  import { app, op, runOp } from '$lib/state.svelte';

  const CHECK_EVERY = 6 * 60 * 60 * 1000;

  let info = $state<SelfUpdate | null>(null);
  let failed = $state(false);
  let checking = $state(false);
  let open = $state(false);
  let installed = $state(false);
  let switching = $state(false);
  let message = $state<{ cls: string; text: string } | null>(null);

  const auto = $derived(!!app.settings?.selfUpdateAuto);
  const version = $derived(info?.current ?? app.info?.version ?? '');

  async function check() {
    checking = true;
    try {
      info = await api.checkSelfUpdate();
      failed = false;
    } catch {
      // Offline or GitHub unreachable: try again at the next check.
      failed = true;
    } finally {
      checking = false;
    }
  }

  $effect(() => {
    check();
    const timer = setInterval(check, CHECK_EVERY);
    return () => clearInterval(timer);
  });

  async function updateNow() {
    const result = await runOp(t('self.updating'), () => api.updateSelf(), ['apt']);
    if (result?.success) installed = true;
  }

  async function setAuto(enabled: boolean) {
    switching = true;
    message = null;
    try {
      app.settings = await api.setSelfUpdateAuto(enabled);
      if (enabled) message = { cls: 'ok', text: t('self.autoOn') };
      await check();
    } catch (e) {
      const text = String(e);
      message = { cls: 'warn', text: text === 'cancelled' ? t('self.autoCancelled') : tMessage(text) };
    } finally {
      switching = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="wrap">
  <button
    class="pill"
    class:new={info?.available && !installed}
    class:ready={installed}
    onclick={() => (open = !open)}
    aria-expanded={open}
  >
    {#if info?.available && !installed}
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M8 13V3M3.5 7.5 8 3l4.5 4.5" />
      </svg>
      {t('self.available', { v: info.latest })}
    {:else if installed}
      {t('self.restart')}
    {:else}
      {t('self.button', { v: version })}
      {#if auto}<span class="dot" title={t('self.auto')}></span>{/if}
    {/if}
  </button>

  {#if open}
    <div class="pop" role="dialog" aria-label={t('self.title')}>
      <strong>{t('self.title')}</strong>

      {#if installed && info}
        <p>{t('self.updated', { v: info.latest })}</p>
        <div class="actions">
          <button class="primary" onclick={() => api.restartApp()}>{t('self.restart')}</button>
        </div>
      {:else if info?.available}
        <p>{t('self.available', { v: info.latest })} <span class="muted">{t('self.current', { v: info.current })}</span></p>
        {#if info.channel === 'apt'}
          <div class="actions">
            <button class="link" onclick={() => openUrl(`${info!.releasesUrl}/tag/${info!.latest}`)}>
              {t('self.notes')}
            </button>
            <button class="primary" onclick={updateNow} disabled={op.running}>
              {#if op.running}<span class="spinner"></span>{/if}
              {t('self.updateNow')}
            </button>
          </div>
        {:else}
          <p class="small muted">{t('self.manual')}</p>
          <div class="actions">
            <button class="primary" onclick={() => openUrl(`${info!.releasesUrl}/tag/${info!.latest}`)}>
              {t('self.download')}
            </button>
          </div>
        {/if}
      {:else}
        <p>{failed || !info ? t('self.unknown') : t('self.upToDate', { v: info.current })}</p>
        <div class="actions">
          <button onclick={check} disabled={checking}>
            {#if checking}<span class="spinner"></span>{/if}
            {t('self.check')}
          </button>
        </div>
      {/if}

      {#if app.info?.helpers}
        <div class="auto">
          <label class="check">
            <input type="checkbox" checked={auto} disabled={switching} onchange={(e) => setAuto(e.currentTarget.checked)} />
            <strong>{t('self.auto')}</strong>
            {#if switching}<span class="spinner"></span>{/if}
          </label>
          <p class="small muted">{t('self.autoText')}</p>
          <p class="small muted">{t('self.viaApt')}</p>
        </div>
      {/if}

      {#if message}<p class="notice {message.cls} small">{message.text}</p>{/if}
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 999px;
    padding: 4px 12px;
    font-size: 12.5px;
    color: var(--muted);
    background: var(--surface);
  }

  .pill.new {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
    font-size: 13px;
    padding: 5px 13px;
    box-shadow: 0 2px 8px rgb(0 0 0 / 15%);
  }

  .pill.new:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .pill.ready {
    background: var(--ok);
    border-color: var(--ok);
    color: #fff;
    font-weight: 600;
  }

  @media (prefers-color-scheme: dark) {
    .pill.new,
    .pill.ready {
      color: #0d1f26;
    }
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }

  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + 8px);
    width: 360px;
    z-index: 10;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 14px 16px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 22%);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 8px 12px;
  }

  .auto {
    display: grid;
    gap: 4px;
    margin-top: 4px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 7px;
  }
</style>
