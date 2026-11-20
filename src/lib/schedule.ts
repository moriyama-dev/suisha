import type { AutoUpdate } from './api';
import { t, type Key } from './i18n.svelte';

const pad = (n: number) => String(n).padStart(2, '0');

export const timeText = (a: AutoUpdate) => `${pad(a.hour)}:${pad(a.minute)}`;

export function hoursText(n: number): string {
  return n === 1 ? t('auto.everyHour') : t('auto.everyHours', { n });
}

/** "Every day at 12:00", "Every 3 hours", ... */
export function scheduleText(a: AutoUpdate): string {
  switch (a.frequency) {
    case 'hours':
      return hoursText(a.everyHours);
    case 'weekly':
      return t('auto.summaryWeekly', { day: t(`day.${a.weekday}` as Key), time: timeText(a) });
    default:
      return t('auto.summaryDaily', { time: timeText(a) });
  }
}

/**
 * When the system timer fires next. Matches the systemd calendar Suisha
 * writes (for hourly schedules: hours divisible by the interval).
 */
export function nextRun(a: AutoUpdate, now = new Date()): Date | null {
  if (!a.enabled) return null;
  const next = new Date(now);
  next.setSeconds(0, 0);
  if (a.frequency === 'hours') {
    next.setMinutes(a.minute);
    for (let i = 0; i < 48; i++) {
      if (next > now && next.getHours() % a.everyHours === 0) return next;
      next.setHours(next.getHours() + 1);
    }
    return null;
  }
  next.setHours(a.hour, a.minute);
  for (let i = 0; i < 8; i++) {
    const dayOk = a.frequency === 'daily' || next.getDay() === a.weekday;
    if (dayOk && next > now) return next;
    next.setDate(next.getDate() + 1);
  }
  return null;
}
