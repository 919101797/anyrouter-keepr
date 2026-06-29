const UPDATE_REMINDER_PREFIX = "anyrouter-keeper:update-reminder:";

export const UPDATE_CHECK_INTERVAL_MS = 60 * 1000;
export const UPDATE_STARTUP_DELAY_MS = 6 * 1000;
export const UPDATE_REMIND_LATER_MS = 12 * 60 * 60 * 1000;

export interface UpdateReminderStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
}

export function shouldShowUpdateReminder(
  version: string,
  now = Date.now(),
  storage: UpdateReminderStorage | null = safeLocalStorage(),
) {
  if (!storage) return true;
  const rawValue = storage.getItem(reminderKey(version));
  if (!rawValue) return true;
  const remindAfter = Number(rawValue);
  return !Number.isFinite(remindAfter) || now >= remindAfter;
}

export function deferUpdateReminder(
  version: string,
  now = Date.now(),
  storage: UpdateReminderStorage | null = safeLocalStorage(),
) {
  storage?.setItem(reminderKey(version), String(now + UPDATE_REMIND_LATER_MS));
}

export function clearUpdateReminder(
  version: string,
  storage: UpdateReminderStorage | null = safeLocalStorage(),
) {
  storage?.removeItem(reminderKey(version));
}

function reminderKey(version: string) {
  return `${UPDATE_REMINDER_PREFIX}${version}`;
}

function safeLocalStorage(): Storage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}
