<script lang="ts">
  import { t, tMessage } from '$lib/i18n.svelte';
  import { op } from '$lib/state.svelte';

  let logEl = $state<HTMLPreElement | undefined>();

  $effect(() => {
    // Keep the newest line in view while output streams in.
    void op.log.length;
    if (logEl && op.showLog) logEl.scrollTop = logEl.scrollHeight;
  });

  const visible = $derived(op.running || op.result !== null || op.error !== '');
  const status = $derived.by(() => {
    if (op.running) return { cls: '', text: t('op.running') };
    if (op.error) return { cls: 'error', text: op.error === 'busy' ? t('common.busy') : tMessage(op.error) };
    if (op.result?.cancelled) return { cls: 'warn', text: t('op.cancelled') };
    if (op.result && !op.result.success) return { cls: 'error', text: t('op.failed') };
    return { cls: 'ok', text: t('op.done') };
  });

  function dismiss() {
    op.result = null;
    op.error = '';
    op.showLog = false;
  }
</script>

{#if visible}
  <section class="panel" aria-live="polite">
    <div class="bar notice {status.cls}">
      {#if op.running}<span class="spinner"></span>{/if}
      <span class="label">
        {#if op.label}<strong>{op.label}</strong> · {/if}{status.text}
        {#if op.running}<span class="muted small"> {t('op.password')}</span>{/if}
      </span>
      <button class="link" onclick={() => (op.showLog = !op.showLog)}>
        {op.showLog ? t('op.hideLog') : t('op.showLog')}
      </button>
      {#if !op.running}
        <button class="link" onclick={dismiss}>{t('common.close')}</button>
      {/if}
    </div>
    {#if op.showLog}
      <pre bind:this={logEl}>{op.log.join('\n')}</pre>
    {/if}
  </section>
{/if}

<style>
  .panel {
    border-top: 1px solid var(--border);
    background: var(--surface);
    padding: 10px 20px 12px;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .label {
    flex: 1;
    min-width: 0;
  }

  pre {
    margin: 10px 0 0;
    max-height: 220px;
    overflow: auto;
    background: var(--surface-2);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 12px;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
