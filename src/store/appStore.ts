import { create } from "zustand";
import { api } from "../lib/api";
import type { StatusPendingAction } from "../lib/statusActions";
import type { ClaudeFingerprintSnapshot, ProxyStatus } from "../lib/fingerprint";
import type {
  ActivityBucket,
  AppStatus,
  ClaudeDetectionLog,
  ClaudeInstallation,
  ClaudeRuntimeConfig,
  ProbeEvent,
  ProfileInput,
  StoredProfile,
  UpstreamModelCatalog,
} from "../lib/types";

interface AppStore {
  profile: StoredProfile | null;
  status: AppStatus | null;
  events: ProbeEvent[];
  activity: ActivityBucket[];
  claudeInstallation: ClaudeInstallation | null;
  claudeRuntimeConfig: ClaudeRuntimeConfig | null;
  upstreamModelCatalog: UpstreamModelCatalog | null;
  upstreamModelsLoading: boolean;
  claudeDetectionLogs: ClaudeDetectionLog[];
  claudeFingerprintSnapshot: ClaudeFingerprintSnapshot | null;
  proxyStatus: ProxyStatus | null;
  anchorTime: number;
  autostartEnabled: boolean;
  loading: boolean;
  busy: boolean;
  pendingAction: StatusPendingAction;
  error: string | null;
  filter: string;
  load: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  saveProfile: (profile: ProfileInput) => Promise<void>;
  refreshClaudeInstallation: () => Promise<void>;
  refreshUpstreamModels: () => Promise<void>;
  testClaudeInstallation: (configuredPath: string) => Promise<void>;
  runProbeNow: () => Promise<void>;
  startScheduler: () => Promise<void>;
  pauseScheduler: () => Promise<void>;
  setFilter: (filter: string) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
  refreshClaudeFingerprint: () => Promise<void>;
  regenerateClaudeFingerprint: () => Promise<void>;
  restoreClaudeFingerprint: (id: string) => Promise<void>;
  deleteClaudeFingerprintHistory: (id: string) => Promise<void>;
  refreshProxyStatus: () => Promise<void>;
  startProxy: () => Promise<void>;
  stopProxy: () => Promise<void>;
  setProxyTarget: (targetOs: string, targetArch: string) => Promise<void>;
  switchAllFingerprints: () => Promise<void>;
}

async function readRuntimeSnapshot() {
  const [status, events, activity] = await Promise.all([
    api.getCurrentStatus(),
    api.listProbeEvents(2000),
    api.getActivitySummary(24),
  ]);
  return { status, events, activity, anchorTime: Date.now() };
}

async function readClaudeSnapshot() {
  const [
    claudeInstallation,
    claudeRuntimeConfig,
    claudeDetectionLogs,
    claudeFingerprintSnapshot,
    proxyStatus,
  ] = await Promise.all([
    api.getClaudeInstallation(),
    api.getClaudeRuntimeConfig(),
    api.listClaudeDetectionLogs(20),
    api.getClaudeFingerprintSnapshot(),
    api.getProxyStatus(),
  ]);
  return {
    claudeInstallation,
    claudeRuntimeConfig,
    claudeDetectionLogs,
    claudeFingerprintSnapshot,
    proxyStatus,
  };
}

async function readFullSnapshot() {
  const [runtime, claude] = await Promise.all([readRuntimeSnapshot(), readClaudeSnapshot()]);
  return { ...runtime, ...claude };
}

export const useAppStore = create<AppStore>((set, get) => ({
  profile: null,
  status: null,
  events: [],
  activity: [],
  claudeInstallation: null,
  claudeRuntimeConfig: null,
  upstreamModelCatalog: null,
  upstreamModelsLoading: false,
  claudeDetectionLogs: [],
  claudeFingerprintSnapshot: null,
  proxyStatus: null,
  anchorTime: Date.now(),
  autostartEnabled: false,
  loading: false,
  busy: false,
  pendingAction: null,
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
      const {
        claudeInstallation,
        claudeRuntimeConfig,
        claudeDetectionLogs,
        claudeFingerprintSnapshot,
        proxyStatus,
      } = await readClaudeSnapshot();
      set({
        profile,
        status,
        events,
        activity,
        autostartEnabled,
        claudeInstallation,
        claudeRuntimeConfig,
        claudeDetectionLogs,
        claudeFingerprintSnapshot,
        proxyStatus,
        anchorTime: Date.now(),
        loading: false,
      });
      void get().refreshUpstreamModels();
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  async refreshStatus() {
    try {
      const snapshot = await readRuntimeSnapshot();
      set({ ...snapshot, error: null });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  async saveProfile(profile) {
    set({ busy: true, pendingAction: "profile", error: null });
    try {
      const saved = await api.saveProfile(profile);
      const {
        claudeInstallation,
        claudeRuntimeConfig,
        claudeDetectionLogs,
        claudeFingerprintSnapshot,
        proxyStatus,
      } = await readClaudeSnapshot();
      set({
        profile: saved,
        claudeInstallation,
        claudeRuntimeConfig,
        claudeDetectionLogs,
        claudeFingerprintSnapshot,
        proxyStatus,
        busy: false,
        pendingAction: null,
      });
      void get().refreshUpstreamModels();
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async refreshClaudeInstallation() {
    set({ busy: true, pendingAction: "claude", error: null });
    try {
      const claudeInstallation = await api.refreshClaudeInstallation();
      const [claudeRuntimeConfig, claudeDetectionLogs] = await Promise.all([
        api.getClaudeRuntimeConfig(),
        api.listClaudeDetectionLogs(20),
      ]);
      set({ claudeInstallation, claudeRuntimeConfig, claudeDetectionLogs, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async refreshUpstreamModels() {
    if (get().upstreamModelsLoading) return;
    set({ upstreamModelsLoading: true });
    try {
      const upstreamModelCatalog = await api.getUpstreamModels();
      set({ upstreamModelCatalog, upstreamModelsLoading: false });
    } catch (error) {
      set({ error: String(error), upstreamModelsLoading: false });
    }
  },

  async testClaudeInstallation(configuredPath) {
    set({ busy: true, pendingAction: "claude_path_test", error: null });
    try {
      const claudeInstallation = await api.testClaudeInstallation(configuredPath);
      set({ claudeInstallation, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async runProbeNow() {
    set({ busy: true, pendingAction: "probe", error: null });
    try {
      await api.runProbeNow();
      const snapshot = await readFullSnapshot();
      set({ ...snapshot, busy: false, pendingAction: null });
    } catch (error) {
      const message = String(error);
      try {
        const snapshot = await readFullSnapshot();
        set({ ...snapshot, error: message, busy: false, pendingAction: null });
      } catch {
        set({ error: message, busy: false, pendingAction: null });
      }
    }
  },

  async startScheduler() {
    set({ busy: true, pendingAction: "start", error: null });
    try {
      await api.startScheduler();
      const snapshot = await readFullSnapshot();
      set({ ...snapshot, busy: false, pendingAction: null });
    } catch (error) {
      const message = String(error);
      try {
        const snapshot = await readFullSnapshot();
        set({ ...snapshot, error: message, busy: false, pendingAction: null });
      } catch {
        set({ error: message, busy: false, pendingAction: null });
      }
    }
  },

  async pauseScheduler() {
    set({ busy: true, pendingAction: "pause", error: null });
    try {
      await api.pauseScheduler();
      set({ busy: false, pendingAction: null });
      await get().refreshStatus();
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async setFilter(filter) {
    set({ filter });
  },

  async setAutostart(enabled) {
    set({ busy: true, pendingAction: "autostart", error: null });
    try {
      await api.setAutostart(enabled);
      set({ autostartEnabled: enabled, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async refreshClaudeFingerprint() {
    set({ busy: true, pendingAction: "fingerprint_refresh", error: null });
    try {
      const [claudeFingerprintSnapshot, proxyStatus] = await Promise.all([
        api.getClaudeFingerprintSnapshot(),
        api.getProxyStatus(),
      ]);
      set({
        claudeFingerprintSnapshot,
        proxyStatus,
        busy: false,
        pendingAction: null,
        error: claudeFingerprintSnapshot.error ?? null,
      });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async regenerateClaudeFingerprint() {
    set({ busy: true, pendingAction: "fingerprint", error: null });
    try {
      const claudeFingerprintSnapshot = await api.regenerateClaudeFingerprint();
      set({
        claudeFingerprintSnapshot,
        busy: false,
        pendingAction: null,
        error: claudeFingerprintSnapshot.error ?? null,
      });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async restoreClaudeFingerprint(id) {
    set({ busy: true, pendingAction: "fingerprint", error: null });
    try {
      const claudeFingerprintSnapshot = await api.restoreClaudeFingerprint(id);
      set({
        claudeFingerprintSnapshot,
        busy: false,
        pendingAction: null,
        error: claudeFingerprintSnapshot.error ?? null,
      });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async deleteClaudeFingerprintHistory(id) {
    set({ busy: true, pendingAction: "fingerprint", error: null });
    try {
      const claudeFingerprintSnapshot = await api.deleteClaudeFingerprintHistory(id);
      set({
        claudeFingerprintSnapshot,
        busy: false,
        pendingAction: null,
        error: claudeFingerprintSnapshot.error ?? null,
      });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async refreshProxyStatus() {
    try {
      const proxyStatus = await api.getProxyStatus();
      set({ proxyStatus, error: proxyStatus.error ?? null });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  async startProxy() {
    set({ busy: true, pendingAction: "proxy", error: null });
    try {
      const proxyStatus = await api.startProxy();
      set({ proxyStatus, error: proxyStatus.error ?? null, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async stopProxy() {
    set({ busy: true, pendingAction: "proxy", error: null });
    try {
      const proxyStatus = await api.stopProxy();
      set({ proxyStatus, error: proxyStatus.error ?? null, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },

  async setProxyTarget(targetOs, targetArch) {
    set({ busy: true, pendingAction: "proxy_target", error: null });
    try {
      const proxyStatus = await api.setProxyTarget(targetOs, targetArch);
      set({ proxyStatus, error: proxyStatus.error ?? null, busy: false, pendingAction: null });
    } catch (error) {
      set({ error: String(error), busy: false, pendingAction: null });
    }
  },
  async switchAllFingerprints() {
    set({ busy: true, pendingAction: "fingerprint", error: null });
    try {
      const result = await api.switchAllFingerprints();
      set({
        claudeFingerprintSnapshot: result.fingerprint,
        proxyStatus: result.proxy,
        busy: false,
        pendingAction: null,
        error: result.fingerprint.error ?? null,
      });
    } catch (error) {
      const message = String(error);
      try {
        const [claudeFingerprintSnapshot, proxyStatus] = await Promise.all([
          api.getClaudeFingerprintSnapshot(),
          api.getProxyStatus(),
        ]);
        set({
          claudeFingerprintSnapshot,
          proxyStatus,
          error: message,
          busy: false,
          pendingAction: null,
        });
      } catch {
        set({ error: message, busy: false, pendingAction: null });
      }
    }
  },
}));
