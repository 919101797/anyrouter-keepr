import { describe, expect, it } from "vitest";
import {
  UPDATE_REMIND_LATER_MS,
  clearUpdateReminder,
  deferUpdateReminder,
  shouldShowUpdateReminder,
  type UpdateReminderStorage,
} from "./updatePolicy";

class MemoryStorage implements UpdateReminderStorage {
  private readonly values = new Map<string, string>();

  getItem(key: string) {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string) {
    this.values.set(key, value);
  }

  removeItem(key: string) {
    this.values.delete(key);
  }
}

describe("update reminder policy", () => {
  it("shows unseen update versions", () => {
    expect(shouldShowUpdateReminder("0.2.0", 1000, new MemoryStorage())).toBe(true);
  });

  it("defers the same update version for the reminder window", () => {
    const storage = new MemoryStorage();

    deferUpdateReminder("0.2.0", 1000, storage);

    expect(shouldShowUpdateReminder("0.2.0", 1000 + UPDATE_REMIND_LATER_MS - 1, storage)).toBe(false);
    expect(shouldShowUpdateReminder("0.2.0", 1000 + UPDATE_REMIND_LATER_MS, storage)).toBe(true);
    expect(shouldShowUpdateReminder("0.3.0", 1000, storage)).toBe(true);
  });

  it("can clear a deferred reminder", () => {
    const storage = new MemoryStorage();

    deferUpdateReminder("0.2.0", 1000, storage);
    clearUpdateReminder("0.2.0", storage);

    expect(shouldShowUpdateReminder("0.2.0", 1000, storage)).toBe(true);
  });
});
