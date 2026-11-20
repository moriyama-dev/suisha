<script lang="ts">
  import { api } from '$lib/api';
  import { t } from '$lib/i18n.svelte';
  import { app, op, runOp } from '$lib/state.svelte';

  const SHOW = 6;
  const s = $derived(app.system);

  function names(list: string[]): string {
    const shown = list.slice(0, SHOW).join(', ');
    return list.length > SHOW ? `${shown}, ${t('system.more', { n: list.length - SHOW })}` : shown;
  }
</script>

{#if s && (s.restartRequired || s.unneeded.length || s.flathubMissing)}
  <div class="notices">
    {#if s.restartRequired}
      <div class="notice warn item">
        <div class="text">
          <strong>{t('system.restartTitle')}</strong>
          <p class="small">
            {s.restartPackages.length
              ? t('system.restartText', { list: names(s.restartPackages) })
              : t('system.restartTextPlain')}
          </p>
        </div>
      </div>
    {/if}
    {#if s.unneeded.length}
      <div class="notice item">
        <div class="text">
          <strong>
            {s.unneeded.length === 1 ? t('system.unneededOne') : t('system.unneededTitle', { n: s.unneeded.length })}
          </strong>
          <p class="small muted">{t('system.unneededText', { list: names(s.unneeded) })}</p>
        </div>
        <button onclick={() => runOp(t('action.clean-up'), () => api.cleanUp(), ['apt'])} disabled={op.running}>
          {t('system.cleanUp')}
        </button>
      </div>
    {/if}
    {#if s.flathubMissing}
      <div class="notice item">
        <div class="text">
          <strong>{t('system.flathubTitle')}</strong>
          <p class="small muted">{t('system.flathubText')}</p>
        </div>
        <button onclick={() => runOp(t('action.add-flathub'), () => api.addFlathub(), ['flatpak'])} disabled={op.running}>
          {t('system.addFlathub')}
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .notices {
    display: grid;
    gap: 8px;
    margin: 14px 0 6px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
</style>
