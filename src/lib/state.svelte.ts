import { listen } from '@tauri-apps/api/event';
import {
  api,
  MANAGER_ORDER,
  type AppInfo,
  type ManagerId,
  type OpResult,
  type Package,
  type PackageRef,
  type Settings,
  type SystemStatus,
} from './api';

type PerManager<T> = Partial<Record<ManagerId, T>>;

export const app = $state({
  info: null as AppInfo | null,
  infoError: '',
  installed: {} as PerManager<Package[]>,
  installedErrors: {} as PerManager<string>,
  installedLoading: false,
  updates: {} as PerManager<Package[]>,
  updatesErrors: {} as PerManager<string>,
  updatesLoading: false,
  updatesCheckedAt: 0,
  settings: null as Settings | null,
  system: null as SystemStatus | null,
  // Set by the sidebar to ask the automatic updates panel to open its editor.
  autoEditRequested: false,
});

export const op = $state({
  running: false,
  label: '',
  log: [] as string[],
  result: null as OpResult | null,
  error: '',
  showLog: false,
});

export function managers(): ManagerId[] {
  const available = app.info?.managers ?? [];
  return MANAGER_ORDER.filter((m) => available.includes(m));
}

export async function loadInfo() {
  try {
    app.info = await api.appInfo();
    app.infoError = '';
  } catch (e) {
    app.infoError = String(e);
  }
}

async function eachManager(
  list: ManagerId[],
  fetch: (m: ManagerId) => Promise<Package[]>,
  target: PerManager<Package[]>,
  errors: PerManager<string>,
) {
  await Promise.all(
    list.map(async (m) => {
      try {
        target[m] = await fetch(m);
        delete errors[m];
      } catch (e) {
        errors[m] = String(e);
        target[m] = [];
      }
    }),
  );
}

export async function loadInstalled(only?: ManagerId[]) {
  app.installedLoading = true;
  try {
    await eachManager(only ?? managers(), api.listInstalled, app.installed, app.installedErrors);
  } finally {
    app.installedLoading = false;
  }
}

export async function loadUpdates(only?: ManagerId[]) {
  app.updatesLoading = true;
  try {
    await eachManager(only ?? managers(), api.listUpdates, app.updates, app.updatesErrors);
    app.updatesCheckedAt = Date.now();
  } finally {
    app.updatesLoading = false;
  }
}

export async function loadSystem() {
  try {
    app.system = await api.systemStatus();
  } catch {
    // The notices are optional; without them the window still works.
  }
}

export async function loadSettings() {
  try {
    app.settings = await api.getSettings();
  } catch {
    // Settings fall back to defaults on the Rust side; nothing to show here.
  }
}

/** Updates that can be installed now (apt's phased and kept-back ones are left out). */
export function updateCount(): number {
  return Object.values(app.updates).reduce((n, l) => n + (l?.filter((p) => !p.held).length ?? 0), 0);
}

/** apt or snap is present, so automatic updates need the password-less permission. */
export function needsAdminAccess(): boolean {
  return managers().some((m) => m === 'apt' || m === 'snap');
}

export function isInstalled(p: { manager: ManagerId; id: string; name?: string }): boolean {
  const list = app.installed[p.manager];
  return !!list?.some((i) => i.id === p.id || (!!p.name && i.name === p.id));
}

let unlisten: (() => void) | null = null;

async function ensureListener() {
  if (unlisten) return;
  unlisten = await listen<string>('op-log', (e) => {
    op.log.push(e.payload);
    if (op.log.length > 4000) op.log.splice(0, op.log.length - 4000);
  });
}

/** Run a change, stream its output, then reload the lists it touched. */
export async function runOp(
  label: string,
  task: () => Promise<OpResult>,
  touched: ManagerId[],
): Promise<OpResult | null> {
  if (op.running) return null;
  await ensureListener();
  op.running = true;
  op.label = label;
  op.log = [];
  op.result = null;
  op.error = '';
  try {
    const result = await task();
    op.result = result;
    if (!result.success) op.showLog = true;
    return result;
  } catch (e) {
    op.error = String(e);
    op.showLog = true;
    return null;
  } finally {
    op.running = false;
    const list = touched.length ? touched : managers();
    await Promise.all([loadUpdates(list), loadInstalled(list), loadSystem()]);
  }
}

export function managersOf(refs: PackageRef[]): ManagerId[] {
  return [...new Set(refs.map((r) => r.manager))];
}
