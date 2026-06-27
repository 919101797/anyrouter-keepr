import { useCallback, useEffect, useRef, useState } from "react";
import {
  checkForAppUpdate,
  normalizeUpdaterError,
  relaunchApp,
  type AppUpdateInfo,
  type PendingAppUpdate,
  type UpdateProgress,
} from "./updater";
import {
  UPDATE_CHECK_INTERVAL_MS,
  UPDATE_STARTUP_DELAY_MS,
  clearUpdateReminder,
  deferUpdateReminder,
  shouldShowUpdateReminder,
} from "./updatePolicy";

export type UpdatePanelState =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
  | "installed"
  | "latest"
  | "error";

export interface AppUpdaterController {
  open: boolean;
  state: UpdatePanelState;
  update: AppUpdateInfo | null;
  progress: UpdateProgress | null;
  error: string | null;
  lastCheckedAt: number | null;
  checkNow: () => Promise<void>;
  installNow: () => Promise<void>;
  remindLater: () => void;
  closePanel: () => void;
  relaunch: () => Promise<void>;
}

export function useAppUpdater(): AppUpdaterController {
  const pendingUpdateRef = useRef<PendingAppUpdate | null>(null);
  const checkingRef = useRef(false);
  const [open, setOpen] = useState(false);
  const [state, setState] = useState<UpdatePanelState>("idle");
  const [update, setUpdate] = useState<AppUpdateInfo | null>(null);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lastCheckedAt, setLastCheckedAt] = useState<number | null>(null);

  const runCheck = useCallback(async (manual: boolean) => {
    if (checkingRef.current) return;
    checkingRef.current = true;
    setError(null);
    setProgress(null);
    if (manual) {
      setOpen(true);
      setState("checking");
    }

    try {
      const nextUpdate = await checkForAppUpdate();
      setLastCheckedAt(Date.now());

      if (!nextUpdate) {
        if (manual) {
          setUpdate(null);
          setState("latest");
        }
        return;
      }

      pendingUpdateRef.current = nextUpdate;
      const nextInfo = toUpdateInfo(nextUpdate);
      setUpdate(nextInfo);
      setState("available");

      if (manual || shouldShowUpdateReminder(nextInfo.version)) {
        setOpen(true);
      }
    } catch (checkError) {
      setError(normalizeUpdaterError(checkError));
      if (manual) {
        setState("error");
        setOpen(true);
      }
    } finally {
      checkingRef.current = false;
    }
  }, []);

  useEffect(() => {
    const startupTimer = window.setTimeout(() => {
      void runCheck(false);
    }, UPDATE_STARTUP_DELAY_MS);
    const intervalTimer = window.setInterval(() => {
      void runCheck(false);
    }, UPDATE_CHECK_INTERVAL_MS);

    return () => {
      window.clearTimeout(startupTimer);
      window.clearInterval(intervalTimer);
    };
  }, [runCheck]);

  const checkNow = useCallback(() => runCheck(true), [runCheck]);

  const installNow = useCallback(async () => {
    const pendingUpdate = pendingUpdateRef.current;
    if (!pendingUpdate) return;

    setError(null);
    setState("downloading");
    setProgress({
      phase: "downloading",
      percent: 0,
      downloadedBytes: 0,
    });

    try {
      await pendingUpdate.downloadAndInstall((nextProgress) => {
        setProgress(nextProgress);
        if (nextProgress.phase === "installing") setState("installing");
      });
      clearUpdateReminder(pendingUpdate.version);
      setState("installed");
      setOpen(true);
    } catch (installError) {
      setError(normalizeUpdaterError(installError));
      setState("error");
      setOpen(true);
    }
  }, []);

  const remindLater = useCallback(() => {
    if (update) deferUpdateReminder(update.version);
    setOpen(false);
  }, [update]);

  const closePanel = useCallback(() => {
    setOpen(false);
  }, []);

  const relaunch = useCallback(async () => {
    await relaunchApp();
  }, []);

  return {
    open,
    state,
    update,
    progress,
    error,
    lastCheckedAt,
    checkNow,
    installNow,
    remindLater,
    closePanel,
    relaunch,
  };
}

function toUpdateInfo(update: AppUpdateInfo): AppUpdateInfo {
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date,
    body: update.body,
  };
}
