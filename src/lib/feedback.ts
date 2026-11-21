import { openUrl } from '@tauri-apps/plugin-opener';
import type { AppInfo } from './api';
import { i18n } from './i18n.svelte';

export const REPO_URL = 'https://github.com/moriyama-dev/suisha';

/** The app details every issue starts with. Nothing personal. */
function details(info: AppInfo | null) {
  const time = new Date().toLocaleString('sv-SE', { timeZone: 'America/Toronto' }).slice(0, 16);
  return [
    '---',
    `- Suisha: ${info?.version ?? 'unknown'}`,
    `- OS: ${info?.os ?? 'unknown'} (${navigator.userAgent.match(/\(([^)]*)\)/)?.[1] ?? ''})`,
    `- Package managers: ${info?.managers.join(', ') || 'none'}`,
    `- Language: ${i18n.lang}`,
    `- Time: ${time} (America/Toronto)`,
  ];
}

function newIssue(lines: string[], info: AppInfo | null, label: string) {
  const body = [...lines, ...details(info)].join('\n');
  openUrl(`${REPO_URL}/issues/new?labels=${label}&body=${encodeURIComponent(body)}`);
}

/** Open a new issue with the details a bug report needs. */
export function reportProblem(info: AppInfo | null) {
  newIssue(
    ['**What happened / 何が起きたか**', '', '', '**Steps to reproduce / 再現手順**', '', ''],
    info,
    'bug',
  );
}

/** Open a new issue for a feature request or an idea. */
export function sendRequest(info: AppInfo | null) {
  newIssue(
    [
      '**What would you like Suisha to do? / Suisha にしてほしいこと**',
      '',
      '',
      '**Why would it help? / それがあると何が楽になるか**',
      '',
      '',
    ],
    info,
    'enhancement',
  );
}

export function openRepo() {
  openUrl(REPO_URL);
}
