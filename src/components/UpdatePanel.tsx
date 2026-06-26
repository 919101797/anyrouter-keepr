import { CheckCircle2, Download, LoaderCircle, RefreshCw, RotateCcw, X } from "lucide-react";
import { Button } from "./ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import type { AppUpdaterController } from "../lib/useAppUpdater";
import { UPDATE_CHECK_INTERVAL_MS } from "../lib/updatePolicy";
import { formatBytes } from "../lib/updater";
import { cn, formatLongDuration } from "../lib/utils";

export function UpdateStatusButton({ updater }: { updater: AppUpdaterController }) {
  const Icon =
    updater.state === "available"
      ? Download
      : updater.state === "checking" || updater.state === "downloading" || updater.state === "installing"
        ? LoaderCircle
        : updater.state === "installed"
          ? CheckCircle2
          : RefreshCw;
  const active =
    updater.state === "available" ||
    updater.state === "checking" ||
    updater.state === "downloading" ||
    updater.state === "installing" ||
    updater.state === "installed";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          className={cn("update-status-button", active && "update-status-button-active")}
          aria-label={updateStatusLabel(updater)}
          onClick={() => {
            void updater.checkNow();
          }}
        >
          <Icon
            className={cn(
              "h-4 w-4",
              (updater.state === "checking" ||
                updater.state === "downloading" ||
                updater.state === "installing") &&
                "animate-spin",
            )}
          />
        </button>
      </TooltipTrigger>
      <TooltipContent>{updateStatusLabel(updater)}</TooltipContent>
    </Tooltip>
  );
}

export function UpdatePanel({ updater }: { updater: AppUpdaterController }) {
  if (!updater.open) return null;

  const busy =
    updater.state === "checking" || updater.state === "downloading" || updater.state === "installing";
  const progressPercent = updater.progress?.percent ?? null;
  const progressStyle = progressPercent == null ? undefined : { width: `${progressPercent}%` };

  return (
    <div className="update-panel-overlay" role="presentation">
      <section className="update-panel" role="dialog" aria-modal="true" aria-labelledby="update-panel-title">
        <button
          type="button"
          className="update-panel-close"
          aria-label="关闭更新面板"
          disabled={busy}
          onClick={updater.closePanel}
        >
          <X className="h-4 w-4" />
        </button>

        <div className="update-panel-scan" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>

        <div className="relative z-10 grid gap-5">
          <div className="flex min-w-0 items-start gap-4">
            <div className="update-panel-icon">
              {updater.state === "installed" ? (
                <CheckCircle2 className="h-5 w-5" />
              ) : updater.state === "checking" ||
                updater.state === "downloading" ||
                updater.state === "installing" ? (
                <LoaderCircle className="h-5 w-5 animate-spin" />
              ) : (
                <Download className="h-5 w-5" />
              )}
            </div>
            <div className="min-w-0">
              <p className="text-xs font-black uppercase tracking-[0.14em] text-[var(--app-text-soft)]">
                GitHub Release
              </p>
              <h2
                id="update-panel-title"
                className="mt-1 text-xl font-black tracking-normal text-[var(--app-text)]"
              >
                {panelTitle(updater)}
              </h2>
              <p className="mt-2 max-w-[560px] text-sm font-semibold leading-6 text-[var(--app-text-muted)]">
                {panelCopy(updater)}
              </p>
            </div>
          </div>

          {updater.update ? (
            <div className="update-version-grid">
              <VersionFact label="当前版本" value={updater.update.currentVersion} />
              <VersionFact label="最新版本" value={updater.update.version} accent />
              <VersionFact label="检查间隔" value={formatLongDuration(UPDATE_CHECK_INTERVAL_MS)} />
            </div>
          ) : null}

          {updater.update?.body ? (
            <div className="update-notes">
              <div className="update-notes-label">更新说明</div>
              <p>{updater.update.body}</p>
            </div>
          ) : null}

          {updater.state === "downloading" ||
          updater.state === "installing" ||
          updater.state === "installed" ? (
            <div className="update-progress-block">
              <div className="mb-2 flex items-center justify-between gap-3 text-xs font-black text-[var(--app-text-muted)]">
                <span>{progressLabel(updater)}</span>
                <span className="mono">
                  {updater.progress?.contentLength
                    ? `${formatBytes(updater.progress.downloadedBytes)} / ${formatBytes(
                        updater.progress.contentLength,
                      )}`
                    : formatBytes(updater.progress?.downloadedBytes)}
                </span>
              </div>
              <div
                className={cn(
                  "update-progress-track",
                  progressPercent == null && "update-progress-indeterminate",
                )}
              >
                <div className="update-progress-fill" style={progressStyle} />
              </div>
            </div>
          ) : null}

          {updater.error ? <div className="update-error">{updater.error}</div> : null}

          <div className="flex flex-col gap-2 sm:flex-row sm:justify-end">
            {updater.state === "available" ? (
              <>
                <Button type="button" variant="ghost" onClick={updater.remindLater}>
                  稍后提醒
                </Button>
                <Button type="button" onClick={() => void updater.installNow()}>
                  <Download />
                  立即更新
                </Button>
              </>
            ) : null}

            {updater.state === "latest" ? (
              <Button type="button" onClick={updater.closePanel}>
                <CheckCircle2 />
                知道了
              </Button>
            ) : null}

            {updater.state === "error" ? (
              <>
                <Button type="button" variant="ghost" onClick={updater.closePanel}>
                  关闭
                </Button>
                <Button type="button" onClick={() => void updater.checkNow()}>
                  <RefreshCw />
                  重新检查
                </Button>
              </>
            ) : null}

            {updater.state === "installed" ? (
              <Button type="button" onClick={() => void updater.relaunch()}>
                <RotateCcw />
                重启完成更新
              </Button>
            ) : null}
          </div>
        </div>
      </section>
    </div>
  );
}

function VersionFact({ label, value, accent = false }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className={cn("update-version-fact", accent && "update-version-fact-accent")}>
      <div>{label}</div>
      <strong>{value}</strong>
    </div>
  );
}

function updateStatusLabel(updater: AppUpdaterController) {
  if (updater.state === "available" && updater.update) return `发现新版本 ${updater.update.version}`;
  if (updater.state === "checking") return "正在检查更新";
  if (updater.state === "downloading") return "正在下载更新";
  if (updater.state === "installing") return "正在安装更新";
  if (updater.state === "installed") return "更新已安装，等待重启";
  if (updater.lastCheckedAt) return "检查更新";
  return "检查更新";
}

function panelTitle(updater: AppUpdaterController) {
  if (updater.state === "checking") return "正在检查更新";
  if (updater.state === "available") return "发现可用更新";
  if (updater.state === "downloading") return "正在下载更新";
  if (updater.state === "installing") return "正在安装更新";
  if (updater.state === "installed") return "更新已准备好";
  if (updater.state === "latest") return "当前已是最新版本";
  if (updater.state === "error") return "更新检查失败";
  return "应用更新";
}

function panelCopy(updater: AppUpdaterController) {
  if (updater.state === "checking") return "正在从 GitHub Releases 拉取 updater 元数据。";
  if (updater.state === "available" && updater.update) {
    return `可以从 ${updater.update.currentVersion} 更新到 ${updater.update.version}，下载包会先经过 Tauri 签名校验再安装。`;
  }
  if (updater.state === "downloading") return "下载进度会根据真实字节数刷新，完成后自动进入安装步骤。";
  if (updater.state === "installing")
    return "安装器正在写入新版本，请保持应用运行。Windows 可能会自动退出完成安装。";
  if (updater.state === "installed") return "更新包已完成安装，重启应用后新版本会生效。";
  if (updater.state === "latest") return "没有发现比当前版本更新的 GitHub Release。";
  if (updater.state === "error") return "可能是网络、GitHub Release 元数据或签名配置还没准备好。";
  return "应用会在后台定期检查 GitHub 是否发布了新版本。";
}

function progressLabel(updater: AppUpdaterController) {
  if (updater.state === "installing") return "正在安装";
  if (updater.state === "installed") return "已完成";
  return "正在下载";
}
