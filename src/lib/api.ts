import { invoke } from '@tauri-apps/api/core';

export type ManagerId = 'apt' | 'snap' | 'flatpak' | 'brew';

export interface Package {
  manager: ManagerId;
  id: string;
  name: string;
  version: string;
  newVersion: string;
  description: string;
  scope: string;
  origin: string;
  classic: boolean;
  /** apt: "phased" or "kept-back" when apt will not install the update yet. */
  held: string;
}

export interface PackageRef {
  manager: ManagerId;
  id: string;
  scope: string;
  origin: string;
  classic: boolean;
}

export interface AppInfo {
  version: string;
  os: 'linux';
  managers: ManagerId[];
  helpers: boolean;
  autoUpdateAllowed: boolean;
}

export interface StepOutcome {
  managers: ManagerId[];
  packages: string[];
  success: boolean;
  cancelled: boolean;
  message: string;
}

export interface OpResult {
  success: boolean;
  cancelled: boolean;
  steps: StepOutcome[];
}

export interface AutoUpdate {
  enabled: boolean;
  frequency: 'hours' | 'daily' | 'weekly';
  everyHours: number;
  weekday: number;
  hour: number;
  minute: number;
  managers: ManagerId[];
  notify: boolean;
}

export interface Settings {
  autoUpdate: AutoUpdate;
  selfUpdateAuto: boolean;
  language: string;
}

export interface HistoryEntry {
  time: string;
  trigger: 'manual' | 'scheduled';
  action: 'install' | 'remove' | 'update' | 'update-all' | 'refresh' | 'clean-up' | 'add-flathub';
  managers: ManagerId[];
  packages: string[];
  success: boolean;
  message: string;
}

export interface SelfUpdate {
  current: string;
  latest: string;
  available: boolean;
  /** "apt": installed from the Suisha package, can update itself. */
  channel: 'apt' | 'manual';
  releasesUrl: string;
  /** The daily update of Suisha may run without a password. */
  autoAllowed: boolean;
}

export interface SystemStatus {
  restartRequired: boolean;
  restartPackages: string[];
  unneeded: string[];
  flathubMissing: boolean;
}

export interface PreviewEntry extends PackageRef {
  name: string;
  status: 'missing' | 'installed' | 'unavailable' | 'invalid';
}

export interface SetupPreview {
  exportedAt: string;
  entries: PreviewEntry[];
}

export const MANAGER_LABELS: Record<ManagerId, string> = {
  apt: 'APT',
  snap: 'Snap',
  flatpak: 'Flatpak',
  brew: 'Homebrew',
};

export const MANAGER_ORDER: ManagerId[] = ['apt', 'snap', 'flatpak', 'brew'];

export function toRef(p: { manager: ManagerId; id: string; scope?: string; origin?: string; classic?: boolean }): PackageRef {
  return {
    manager: p.manager,
    id: p.id,
    scope: p.scope ?? '',
    origin: p.origin ?? '',
    classic: p.classic ?? false,
  };
}

export const api = {
  appInfo: () => invoke<AppInfo>('app_info'),
  listInstalled: (manager: ManagerId) => invoke<Package[]>('list_installed', { manager }),
  listUpdates: (manager: ManagerId) => invoke<Package[]>('list_updates', { manager }),
  search: (manager: ManagerId, query: string) => invoke<Package[]>('search', { manager, query }),
  runAction: (action: 'install' | 'remove' | 'update', packages: PackageRef[]) =>
    invoke<OpResult>('run_action', { action, packages }),
  updateAll: (managers: ManagerId[]) => invoke<OpResult>('update_all', { managers }),
  refresh: () => invoke<OpResult>('refresh'),
  exportSetup: (path: string) => invoke<number>('export_setup', { path }),
  readSetup: (path: string) => invoke<SetupPreview>('read_setup', { path }),
  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) =>
    invoke<{ settings: Settings; warning: string | null }>('save_settings', { settings }),
  setLanguage: (language: string) => invoke<void>('set_language', { language }),
  grantAutoUpdateAccess: () => invoke<boolean>('grant_auto_update_access'),
  getHistory: () => invoke<HistoryEntry[]>('get_history'),
  checkSelfUpdate: () => invoke<SelfUpdate>('check_self_update'),
  updateSelf: () => invoke<OpResult>('update_self'),
  setSelfUpdateAuto: (enabled: boolean) => invoke<Settings>('set_self_update_auto', { enabled }),
  systemStatus: () => invoke<SystemStatus>('system_status'),
  cleanUp: () => invoke<OpResult>('clean_up'),
  addFlathub: () => invoke<OpResult>('add_flathub'),
  restartApp: () => invoke<void>('restart_app'),
};
