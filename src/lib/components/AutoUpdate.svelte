<script lang="ts">
  import { onMount } from 'svelte';
  import { api, MANAGER_LABELS, type AutoUpdate, type ManagerId } from '$lib/api';
  import { t, tMessage, type Key } from '$lib/i18n.svelte';
  import { scheduleText, nextRun, timeText, hoursText } from '$lib/schedule';
  import { app, autoManagers, isMac, loadInfo, loadSettings, managers, needsAdminAccess } from '$lib/state.svelte';
  import { formatTime } from '$lib/time';
  import Wheel from './Wheel.svelte';

  let draft = $state<AutoUpdate | null>(null);
  let saving = $state(false);
  let granting = $state(false);
  let message = $state<{ cls: string; text: string } | null>(null);
  let lastRunTime = $state('');
  // Formatted here so the text follows the display language.
  const lastRun = $derived(lastRunTime ? formatTime(lastRunTime) : '');

  const a = $derived(app.settings?.autoUpdate);
  // Ticks so that "Next" moves on once its time has passed.
  let now = $state(Date.now());
  const next = $derived(a ? nextRun(a, new Date(now)) : null);
  const admin = $derived(needsAdminAccess());
  const allowed = $derived(!!app.info?.autoUpdateAllowed);
  // The password dialog appears on save when apt or snap is included and not yet allowed.
  const draftNeedsPassword = $derived(
    !!draft &&
      admin &&
      !allowed &&
      !!app.info?.helpers &&
      draft.managers.some((m) => m === 'apt' || m === 'snap'),
  );

  const FREQUENCIES = ['h1', 'h3', 'h6', 'h12', 'daily', 'weekly'] as const;
  type FrequencyChoice = (typeof FREQUENCIES)[number];

  function choiceOf(x: AutoUpdate): FrequencyChoice {
    if (x.frequency === 'hours') return `h${x.everyHours}` as FrequencyChoice;
    return x.frequency;
  }

  function setChoice(c: string) {
    if (!draft) return;
    if (c.startsWith('h')) {
      draft.frequency = 'hours';
      draft.everyHours = Number(c.slice(1));
    } else {
      draft.frequency = c as 'daily' | 'weekly';
    }
  }

  function choiceLabel(c: FrequencyChoice) {
    if (c === 'daily') return t('auto.daily');
    if (c === 'weekly') return t('auto.weekly');
    return hoursText(Number(c.slice(1)));
  }

  async function loadLastRun() {
    const history = await api.getHistory();
    const last = history.find((h) => h.trigger === 'scheduled');
    lastRunTime = last ? last.time : '';
  }

  onMount(async () => {
    if (!app.settings) await loadSettings();
    await loadLastRun();
  });

  // Scheduled runs happen while the window stays open too: keep "Next" and
  // "Last run" current, and catch up when the window comes back to the front.
  $effect(() => {
    const refresh = () => {
      now = Date.now();
      loadLastRun().catch(() => {});
    };
    const tick = setInterval(() => (now = Date.now()), 60 * 1000);
    const reread = setInterval(refresh, 5 * 60 * 1000);
    window.addEventListener('focus', refresh);
    return () => {
      clearInterval(tick);
      clearInterval(reread);
      window.removeEventListener('focus', refresh);
    };
  });

  let panel = $state<HTMLElement | null>(null);

  // The sidebar card asks for the editor; settings may still be loading.
  $effect(() => {
    if (!app.autoEditRequested || !app.settings) return;
    app.autoEditRequested = false;
    if (!draft && !saving) edit();
    panel?.scrollIntoView({ block: 'nearest' });
  });

  function edit() {
    if (!app.settings) return;
    const current = app.settings.autoUpdate;
    draft = {
      ...current,
      enabled: true,
      managers: (current.managers.length ? current.managers : autoManagers()).filter((m) => m !== 'mas'),
    };
    message = null;
  }

  async function save(nextSettings: AutoUpdate) {
    if (!app.settings) return;
    saving = true;
    message = null;
    try {
      // Saving every available manager is stored as "all", so managers added later are included.
      const all = autoManagers().every((m) => nextSettings.managers.includes(m));
      const res = await api.saveSettings({
        ...app.settings,
        autoUpdate: { ...nextSettings, managers: all ? [] : nextSettings.managers },
      });
      app.settings = res.settings;
      draft = null;
      await loadInfo();
      if (res.warning) message = { cls: 'warn', text: tMessage(res.warning) };
      else if (nextSettings.enabled) message = { cls: 'ok', text: t('auto.saved') };
    } catch (e) {
      message = { cls: 'error', text: String(e) };
    } finally {
      saving = false;
    }
  }

  async function grant() {
    granting = true;
    message = null;
    try {
      const ok = await api.grantAutoUpdateAccess();
      await loadInfo();
      message = ok
        ? { cls: 'ok', text: t('auto.adminRegistered') }
        : { cls: 'warn', text: t('auto.accessCancelled') };
    } catch (e) {
      const text = String(e);
      message = { cls: 'warn', text: text === 'cancelled' ? t('auto.accessCancelled') : tMessage(text) };
    } finally {
      granting = false;
    }
  }

  function toggle() {
    if (!a) return;
    if (a.enabled) save({ ...a, enabled: false });
    else edit();
  }

  function setTime(value: string) {
    if (!draft) return;
    const [h, m] = value.split(':').map(Number);
    if (Number.isInteger(h) && Number.isInteger(m)) {
      draft.hour = h;
      draft.minute = m;
    }
  }

  function toggleManager(id: ManagerId, on: boolean) {
    if (!draft) return;
    draft.managers = on ? [...draft.managers, id] : draft.managers.filter((m) => m !== id);
  }
</script>

<section class="hero" class:on={!!a?.enabled} aria-labelledby="auto-title" bind:this={panel}>
  <div class="head">
    <div class="mark"><Wheel size={46} spinning={!!a?.enabled} /></div>
    <div class="main">
      <h2 id="auto-title">
        {t('auto.title')}
        {#if a}
          <span class="badge {a.enabled ? 'ok' : ''}">{a.enabled ? t('auto.on') : t('auto.off')}</span>
        {/if}
      </h2>
      {#if a?.enabled}
        <p>
          <button class="schedule" onclick={edit} disabled={saving || !!draft} title={t('auto.change')}>
            {scheduleText(a)}
          </button>
        </p>
        <p class="muted small">
          {#if next}{t('auto.next', { time: formatTime(next.toISOString()) })}{/if}
          {#if next && lastRun} · {/if}
          {#if lastRun}{t('auto.lastRun', { time: lastRun })}{/if}
        </p>
        <p class="muted small">{t('auto.closeOk')}</p>
      {:else}
        <p class="muted">{t('auto.heroOff')}</p>
      {/if}
    </div>
    {#if a && !draft}
      <div class="controls">
        {#if a.enabled}
          <button onclick={edit} disabled={saving}>{t('auto.change')}</button>
        {:else}
          <button class="primary" onclick={edit} disabled={saving}>{t('auto.turnOn')}</button>
        {/if}
        <label class="switch" title={a.enabled ? t('auto.on') : t('auto.off')}>
          <input type="checkbox" checked={a.enabled} disabled={saving} onchange={toggle} />
          <span aria-hidden="true"></span>
          <span class="sr">{t('auto.title')}</span>
        </label>
      </div>
    {/if}
  </div>

  {#if admin && a?.enabled && !draft}
    {#if !app.info?.helpers}
      <div class="admin warn">
        <span class="icon" aria-hidden="true">!</span>
        <p>{t('auto.noHelpers')}</p>
      </div>
    {:else if allowed}
      <div class="admin ok">
        <span class="icon" aria-hidden="true">✓</span>
        <p>{t('auto.adminAllowed')}</p>
      </div>
    {:else}
      <div class="admin warn">
        <span class="icon" aria-hidden="true">!</span>
        <div class="text">
          <strong>{t('auto.adminTitleNeeded')}</strong>
          <p>{t('auto.adminNeededText')}</p>
        </div>
        <button class="primary" onclick={grant} disabled={granting}>
          {#if granting}<span class="spinner"></span>{/if}
          {t('auto.adminRegister')}
        </button>
      </div>
    {/if}
  {/if}

  {#if draft}
    <div class="editor">
      <div class="fields">
        <label>
          <span>{t('auto.frequency')}</span>
          <select value={choiceOf(draft)} onchange={(e) => setChoice(e.currentTarget.value)}>
            {#each FREQUENCIES as c (c)}
              <option value={c}>{choiceLabel(c)}</option>
            {/each}
          </select>
        </label>
        {#if draft.frequency === 'weekly'}
          <label>
            <span>{t('auto.day')}</span>
            <select bind:value={draft.weekday}>
              {#each [1, 2, 3, 4, 5, 6, 0] as d (d)}
                <option value={d}>{t(`day.${d}` as Key)}</option>
              {/each}
            </select>
          </label>
        {/if}
        {#if draft.frequency !== 'hours'}
          <label>
            <span>{t('auto.time')}</span>
            <input type="time" value={timeText(draft)} onchange={(e) => setTime(e.currentTarget.value)} />
          </label>
        {/if}
      </div>
      {#if draft.frequency === 'hours'}
        <p class="muted small">{t('auto.hoursNote')}</p>
      {/if}

      <fieldset>
        <legend>{t('auto.what')}</legend>
        {#each autoManagers() as m (m)}
          <label class="check">
            <input
              type="checkbox"
              checked={draft.managers.includes(m)}
              onchange={(e) => toggleManager(m, e.currentTarget.checked)}
            />
            {MANAGER_LABELS[m]}
          </label>
        {/each}
      </fieldset>
      {#if managers().includes('mas')}
        <p class="muted small">{t('auto.masNote')}</p>
      {/if}

      <label class="check">
        <input type="checkbox" bind:checked={draft.notify} />
        {t('auto.notify')}
      </label>

      <p class="muted small">{isMac() ? t('auto.explainMac') : t('auto.explainLinux')}</p>

      {#if draftNeedsPassword}
        <div class="admin warn">
          <span class="icon" aria-hidden="true">!</span>
          <div class="text">
            <strong>{t('auto.adminTitleNeeded')}</strong>
            <p>{t('auto.passwordNext')}</p>
          </div>
        </div>
      {/if}

      <div class="actions">
        <button onclick={() => (draft = null)} disabled={saving}>{t('common.cancel')}</button>
        <button class="primary" onclick={() => draft && save(draft)} disabled={saving || !draft.managers.length}>
          {#if saving}<span class="spinner"></span>{/if}
          {draftNeedsPassword ? t('auto.saveWithPassword') : t('common.save')}
        </button>
      </div>
    </div>
  {/if}

  {#if message}
    <p class="notice {message.cls} small msg">{message.text}</p>
  {/if}
</section>

<style>
  .hero {
    background: var(--surface);
    border: 1px solid var(--border);
    border-left: 4px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow);
    padding: 20px 22px;
    margin-bottom: 36px;
  }

  .hero.on {
    background: linear-gradient(135deg, var(--accent-soft), var(--surface) 70%);
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
    border-left-color: var(--accent);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .mark {
    flex: none;
    width: 62px;
    height: 62px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 17px;
  }

  .schedule {
    padding: 0;
    border: none;
    background: none;
    font-size: 16px;
    font-weight: 600;
    color: var(--accent-strong);
    cursor: pointer;
  }

  .schedule:hover:not(:disabled) {
    background: none;
    text-decoration: underline;
  }

  .schedule:disabled {
    opacity: 1;
    cursor: default;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .admin {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    padding: 12px 14px;
    border-radius: 10px;
  }

  .admin.ok {
    background: var(--ok-soft);
    color: var(--ok);
  }

  .admin.warn {
    background: var(--warn-soft);
    color: var(--warn);
    border: 1px solid color-mix(in srgb, var(--warn) 45%, transparent);
  }

  .admin .text {
    flex: 1;
    display: grid;
    gap: 2px;
  }

  .admin p {
    flex: 1;
  }

  .admin .icon {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid currentColor;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 12px;
  }

  .switch {
    position: relative;
    width: 46px;
    height: 26px;
    flex: none;
    cursor: pointer;
  }

  .switch input {
    position: absolute;
    opacity: 0;
    inset: 0;
    margin: 0;
    cursor: pointer;
  }

  .switch span[aria-hidden] {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: var(--border);
    transition: background 0.15s;
  }

  .switch span[aria-hidden]::after {
    content: '';
    position: absolute;
    width: 20px;
    height: 20px;
    top: 3px;
    left: 3px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.15s;
  }

  .switch input:checked + span {
    background: var(--accent);
  }

  .switch input:checked + span::after {
    transform: translateX(20px);
  }

  .switch input:focus-visible + span {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .editor {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
    display: grid;
    gap: 12px;
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }

  .fields label {
    display: grid;
    gap: 4px;
  }

  .fields label span,
  legend {
    font-size: 12.5px;
    color: var(--muted);
  }

  fieldset {
    border: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
  }

  legend {
    padding: 0;
    margin-bottom: 4px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .msg {
    margin-top: 14px;
  }
</style>
