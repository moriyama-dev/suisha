import { openUrl } from '@tauri-apps/plugin-opener';
import type { AppInfo } from './api';
import { i18n } from './i18n.svelte';

export const REPO_URL = 'https://github.com/moriyama-dev/suisha';

/** Open a new issue with the details a bug report needs, and nothing personal. */
export function reportProblem(info: AppInfo | null) {
  const time = new Date().toLocaleString('sv-SE', { timeZone: 'America/Toronto' }).slice(0, 16);
  const body = [
    '**What happened / 何が起きたか**',
    '',
    '',
    '**Steps to reproduce / 再現手順**',
    '',
    '',
    '---',
    `- Suisha: ${info?.version ?? 'unknown'}`,
    `- OS: ${info?.os ?? 'unknown'} (${navigator.userAgent.match(/\(([^)]*)\)/)?.[1] ?? ''})`,
    `- Package managers: ${info?.managers.join(', ') || 'none'}`,
    `- Language: ${i18n.lang}`,
    `- Time: ${time} (America/Toronto)`,
  ].join('\n');
  openUrl(`${REPO_URL}/issues/new?body=${encodeURIComponent(body)}`);
}

export function openRepo() {
  openUrl(REPO_URL);
}
