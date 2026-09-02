import { ArrowRight, CheckCircle2, Download, LoaderCircle, RefreshCw, RotateCcw, X } from "lucide-react";
import { Button } from "./ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import type { AppUpdaterController } from "../lib/useAppUpdater";
import { formatBytes, formatUpdateDetails } from "../lib/updater";
import { cn } from "../lib/utils";

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
              <h2
                id="update-panel-title"
                className="text-xl font-black tracking-normal text-[var(--app-text)]"
              >
                {panelTitle(updater)}
              </h2>
            </div>
          </div>

          {updater.update ? (
            <div className="update-version-flow">
              <VersionFact label="当前版本" value={updater.update.currentVersion} />
              <div className="update-version-arrow" aria-hidden="true">
                <ArrowRight className="h-4 w-4" />
              </div>
              <VersionFact label="更新版本" value={updater.update.version} accent />
            </div>
          ) : null}

          {updater.update ? (
            <div className="update-notes">
              <div className="update-notes-label">更新详情</div>
              <p>{formatUpdateDetails(updater.update.body)}</p>
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
  if (updater.state === "available") return "版本更新";
  if (updater.state === "downloading") return "正在下载更新";
  if (updater.state === "installing") return "正在安装更新";
  if (updater.state === "installed") return "更新已准备好";
  if (updater.state === "latest") return "当前已是最新版本";
  if (updater.state === "error") return "更新检查失败";
  return "版本更新";
}

function progressLabel(updater: AppUpdaterController) {
  if (updater.state === "installing") return "正在安装";
  if (updater.state === "installed") return "已完成";
  return "正在下载";
}
