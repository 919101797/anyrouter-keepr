import type { DownloadEvent } from "@tauri-apps/plugin-updater";

export type UpdatePhase = "idle" | "downloading" | "installing" | "installed";

export interface AppUpdateInfo {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
}

export interface UpdateProgress {
  phase: UpdatePhase;
  percent: number | null;
  downloadedBytes: number;
  contentLength?: number;
}

export interface PendingAppUpdate extends AppUpdateInfo {
  downloadAndInstall: (onProgress: (progress: UpdateProgress) => void) => Promise<void>;
  close: () => Promise<void>;
}

export interface ProgressAccumulator {
  downloadedBytes: number;
  contentLength?: number;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const UPDATE_CHECK_TIMEOUT_MS = 30_000;
const UPDATE_DOWNLOAD_TIMEOUT_MS = 10 * 60_000;
const RELEASE_METADATA_ERROR_PATTERNS = [
  /could not fetch a valid release json/i,
  /release json/i,
  /latest\.json/i,
  /404|not found/i,
];

export async function checkForAppUpdate(): Promise<PendingAppUpdate | null> {
  const mockUpdate = createBrowserMockUpdate();
  if (mockUpdate) return mockUpdate;
  if (!inTauri) return null;

  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check({ timeout: UPDATE_CHECK_TIMEOUT_MS });
  if (!update) return null;

  return {
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date,
    body: update.body,
    async downloadAndInstall(onProgress) {
      let accumulator: ProgressAccumulator = { downloadedBytes: 0 };

      onProgress({
        phase: "downloading",
        percent: 0,
        downloadedBytes: 0,
      });

      await update.downloadAndInstall(
        (event) => {
          const next = reduceDownloadProgress(accumulator, event);
          accumulator = {
            downloadedBytes: next.downloadedBytes,
            contentLength: next.contentLength,
          };
          onProgress(next);
        },
        { timeout: UPDATE_DOWNLOAD_TIMEOUT_MS },
      );

      onProgress({
        phase: "installed",
        percent: 100,
        downloadedBytes: accumulator.downloadedBytes,
        contentLength: accumulator.contentLength,
      });
    },
    close: () => update.close(),
  };
}

export async function relaunchApp() {
  if (!inTauri) return;
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

export function reduceDownloadProgress(
  accumulator: ProgressAccumulator,
  event: DownloadEvent,
): UpdateProgress {
  if (event.event === "Started") {
    return {
      phase: "downloading",
      percent: 0,
      downloadedBytes: 0,
      contentLength: event.data.contentLength,
    };
  }

  if (event.event === "Progress") {
    const downloadedBytes = accumulator.downloadedBytes + event.data.chunkLength;
    const contentLength = accumulator.contentLength;
    const percent =
      contentLength && contentLength > 0
        ? Math.min(99, Math.max(0, Math.round((downloadedBytes / contentLength) * 100)))
        : null;

    return {
      phase: "downloading",
      percent,
      downloadedBytes,
      contentLength,
    };
  }

  return {
    phase: "installing",
    percent: 100,
    downloadedBytes: accumulator.downloadedBytes,
    contentLength: accumulator.contentLength,
  };
}

export function formatBytes(value?: number) {
  if (!value || value <= 0) return "未知大小";
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

export function normalizeUpdaterError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const compactMessage = message.trim();

  if (isReleaseMetadataError(compactMessage)) {
    return "无法读取 Cloudflare Pages 更新元数据。请确认 latest.json 和安装包资产已经随 release tag 发布，并且可以匿名下载。";
  }

  return compactMessage || "更新失败，请稍后重试。";
}

function isReleaseMetadataError(message: string) {
  const matched = RELEASE_METADATA_ERROR_PATTERNS.some((pattern) => pattern.test(message));
  return matched && /cloudflare|pages|release|json|latest|404|not found/i.test(message);
}

function createBrowserMockUpdate(): PendingAppUpdate | null {
  if (inTauri || typeof window === "undefined") return null;
  const params = new URLSearchParams(window.location.search);
  if (params.get("mockUpdate") !== "1") return null;

  return {
    version: "0.1.8-test",
    currentVersion: "0.1.7",
    date: new Date().toISOString(),
    body: "测试更新：验证发现更新、下载进度、安装完成和重启按钮状态。",
    async downloadAndInstall(onProgress) {
      const contentLength = 12 * 1024 * 1024;
      let downloadedBytes = 0;
      onProgress({ phase: "downloading", percent: 0, downloadedBytes, contentLength });

      for (let index = 0; index < 8; index += 1) {
        await wait(90);
        downloadedBytes = Math.min(contentLength, downloadedBytes + contentLength / 8);
        onProgress({
          phase: "downloading",
          percent: Math.min(99, Math.round((downloadedBytes / contentLength) * 100)),
          downloadedBytes,
          contentLength,
        });
      }

      await wait(120);
      onProgress({ phase: "installing", percent: 100, downloadedBytes: contentLength, contentLength });
      await wait(180);
      onProgress({ phase: "installed", percent: 100, downloadedBytes: contentLength, contentLength });
    },
    async close() {
      return;
    },
  };
}

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}
