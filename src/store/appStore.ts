import { create } from "zustand";
import { api } from "../lib/api";
import type {
  ActivityBucket,
  AppStatus,
  ClaudeDetectionLog,
  ClaudeInstallation,
  ClaudeRuntimeConfig,
  ProbeEvent,
  ProfileInput,
  StoredProfile,
} from "../lib/types";

interface AppStore {
  profile: StoredProfile | null;
  status: AppStatus | null;
  events: ProbeEvent[];
  activity: ActivityBucket[];
  claudeInstallation: ClaudeInstallation | null;
  claudeRuntimeConfig: ClaudeRuntimeConfig | null;
  claudeDetectionLogs: ClaudeDetectionLog[];
  anchorTime: number;
  autostartEnabled: boolean;
  loading: boolean;
  busy: boolean;
  error: string | null;
  filter: string;
  load: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  saveProfile: (profile: ProfileInput) => Promise<void>;
  refreshClaudeInstallation: () => Promise<void>;
  runProbeNow: () => Promise<void>;
  startScheduler: () => Promise<void>;
  pauseScheduler: () => Promise<void>;
  setFilter: (filter: string) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  profile: null,
  status: null,
  events: [],
  activity: [],
  claudeInstallation: null,
  claudeRuntimeConfig: null,
  claudeDetectionLogs: [],
  anchorTime: Date.now(),
  autostartEnabled: false,
  loading: false,
  busy: false,
  error: null,
  filter: "all",

  async load() {
    set({ loading: true, error: null });
    try {
      const [profile, status, events, activity, autostartEnabled] = await Promise.all([
        api.getProfile(),
        api.getCurrentStatus(),
        api.listProbeEvents(2000),
        api.getActivitySummary(24),
        api.isAutostartEnabled(),
      ]);
      const [claudeInstallation, claudeRuntimeConfig, claudeDetectionLogs] = await Promise.all([
        api.getClaudeInstallation(),
        api.getClaudeRuntimeConfig(),
        api.listClaudeDetectionLogs(20),
      ]);
      set({
        profile,
        status,
        events,
        activity,
        autostartEnabled,
        claudeInstallation,
        claudeRuntimeConfig,
        claudeDetectionLogs,
        anchorTime: Date.now(),
        loading: false,
      });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  async refreshStatus() {
    try {
      const [status, events, activity] = await Promise.all([
        api.getCurrentStatus(),
        api.listProbeEvents(2000),
        api.getActivitySummary(24),
      ]);
      set({ status, events, activity, anchorTime: Date.now(), error: null });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  async saveProfile(profile) {
    set({ busy: true, error: null });
    try {
      const saved = await api.saveProfile(profile);
      const [claudeInstallation, claudeRuntimeConfig, claudeDetectionLogs] = await Promise.all([
        api.getClaudeInstallation(),
        api.getClaudeRuntimeConfig(),
        api.listClaudeDetectionLogs(20),
      ]);
      set({ profile: saved, claudeInstallation, claudeRuntimeConfig, claudeDetectionLogs, busy: false });
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },

  async refreshClaudeInstallation() {
    set({ busy: true, error: null });
    try {
      const claudeInstallation = await api.refreshClaudeInstallation();
      const [claudeRuntimeConfig, claudeDetectionLogs] = await Promise.all([
        api.getClaudeRuntimeConfig(),
        api.listClaudeDetectionLogs(20),
      ]);
      set({ claudeInstallation, claudeRuntimeConfig, claudeDetectionLogs, busy: false });
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },

  async runProbeNow() {
    set({ busy: true, error: null });
    try {
      await api.runProbeNow();
      set({ busy: false });
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },

  async startScheduler() {
    set({ busy: true, error: null });
    try {
      await api.startScheduler();
      set({ busy: false });
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },

  async pauseScheduler() {
    set({ busy: true, error: null });
    try {
      await api.pauseScheduler();
      set({ busy: false });
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },

  async setFilter(filter) {
    set({ filter });
  },

  async setAutostart(enabled) {
    set({ busy: true, error: null });
    try {
      await api.setAutostart(enabled);
      set({ autostartEnabled: enabled, busy: false });
    } catch (error) {
      set({ error: String(error), busy: false });
    }
  },
}));
