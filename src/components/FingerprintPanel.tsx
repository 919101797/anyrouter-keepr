import { History, Play, RefreshCw, RotateCcw, Server, Shuffle, Square, Trash2 } from "lucide-react";
import type {
  ClaudeFingerprintSnapshot,
  ClaudeFingerprintHistoryEntry,
  ProxyStatus,
} from "../lib/fingerprint";
import {
  FINGERPRINT_MATRIX,
  fingerprintSourceLabel,
  fingerprintsMatch,
  fingerprintItemDetail,
  shortDeviceId,
} from "../lib/fingerprint";
import { formatClock, formatRelativeTime } from "../lib/utils";
import type { StatusPendingAction } from "../lib/statusActions";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";

interface FingerprintPanelProps {
  snapshot: ClaudeFingerprintSnapshot | null;
  proxyStatus: ProxyStatus | null;
  busy: boolean;
  error?: string | null;
  pendingAction?: StatusPendingAction;
  onRefresh: () => void;
  onRestore: (id: string) => void;
  onDelete: (id: string) => void;
  onStartProxy: () => void;
  onStopProxy: () => void;
  onSetProxyTarget: (targetOs: string, targetArch: string) => void;
  onSwitchAll: () => void;
}

const TARGET_OS_OPTIONS = ["Windows", "Linux", "MacOS"] as const;
const TARGET_ARCH_OPTIONS = ["x64", "arm64"] as const;

export function FingerprintPanel({
  snapshot,
  proxyStatus,
  busy,
  error,
  pendingAction = null,
  onRefresh,
  onRestore,
  onDelete,
  onStartProxy,
  onStopProxy,
  onSetProxyTarget,
  onSwitchAll,
}: FingerprintPanelProps) {
  const current = snapshot?.current ?? null;
  const history = snapshot?.history ?? [];
  const fingerprintBusy = busy && pendingAction === "fingerprint";
  const refreshBusy = busy && pendingAction === "fingerprint_refresh";
  const proxyBusy = busy && (pendingAction === "proxy" || pendingAction === "proxy_target");
  const proxyRunning = proxyStatus?.running ?? false;
  const targetOs = proxyStatus?.target_os ?? "Windows";
  const targetArch = proxyStatus?.target_arch ?? "x64";
  const upstreamUrl = proxyStatus?.upstream_url?.trim() || "https://anyrouter.top";
  const upstreamMode = proxyStatus?.dynamic_upstream ? "跟随 cc-switch" : "固定上游";
  const visibleError = error || proxyStatus?.error || snapshot?.error;

  const handleTargetChange = (os: string, arch: string) => {
    onSetProxyTarget(os, arch);
  };

  return (
    <div className="panel-ring overflow-hidden rounded-[8px] border border-[#d2ded7] bg-white">
      <div className="border-b border-[#dce7e1] px-5 py-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 className="text-sm font-bold text-[#121815]">AnyRouter 指纹</h2>
            <p className="mt-1 text-xs font-medium text-[#66796f]">
              基于 2026-07-02 交叉 replay 验证 (5 项指纹)
            </p>
          </div>
          <div className="flex items-center gap-2">
            {proxyRunning ? (
              <Badge variant="success">
                <Server />
                代理 ON
              </Badge>
            ) : null}
            <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onRefresh}>
              <RefreshCw className={refreshBusy ? "animate-spin" : ""} />
              刷新
            </Button>
          </div>
        </div>
      </div>

      <div className="divide-y divide-[#edf3ef]">
        {visibleError ? (
          <div className="px-5 pt-4">
            <div className="rounded-[7px] border border-[#f0c36f] bg-[#fff7dc] px-3 py-2 text-xs font-semibold leading-5 text-[#7a570f]">
              {visibleError}
            </div>
          </div>
        ) : null}

        <section className="px-5 py-4">
          <Button
            type="button"
            className="w-full"
            size="default"
            disabled={busy || !current}
            onClick={onSwitchAll}
          >
            <Shuffle className={fingerprintBusy ? "animate-spin" : ""} />
            一键切换全部指纹
          </Button>
          <p className="mt-2 text-center text-xs font-medium text-[#66796f]">
            随机 OS (Windows/MacOS/Linux) × 随机 Arch (x64/arm64) × 随机 device_id
          </p>
        </section>

        <section className="px-5 py-4">
          <div className="mb-3">
            <h3 className="text-xs font-bold uppercase tracking-[0.1em] text-[#405149]">指纹项明细</h3>
          </div>

          <div className="space-y-1.5">
            {current ? (
              FINGERPRINT_MATRIX.map((item) => (
                <div key={item.key} className="rounded-[7px] border border-[#dce7e1] bg-[#f8fbf9] px-3 py-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-xs font-bold text-[#17211d]">{item.label}</span>
                    <span className="shrink-0 text-[10px] font-medium text-[#66796f]">{item.location}</span>
                  </div>
                  <div className="mt-1 text-[11px] font-medium leading-4 text-[#5b6f64]">
                    {fingerprintItemDetail(item.key, current, proxyStatus)}
                  </div>
                </div>
              ))
            ) : (
              <div className="rounded-[7px] border border-dashed border-[#cbd8d1] px-3 py-4 text-center text-xs font-semibold text-[#66796f]">
                指纹数据未加载
              </div>
            )}
          </div>
        </section>

        <section className="px-5 py-4">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div>
              <h3 className="text-xs font-bold uppercase tracking-[0.1em] text-[#405149]">指纹代理</h3>
              <p className="mt-0.5 text-xs font-medium text-[#66796f]">
                重写 header + 剥离 context_management
              </p>
            </div>
            <div className="flex items-center gap-2">
              {proxyRunning ? (
                <Button type="button" size="sm" variant="outline" disabled={proxyBusy} onClick={onStopProxy}>
                  <Square />
                  停止
                </Button>
              ) : (
                <Button type="button" size="sm" disabled={proxyBusy} onClick={onStartProxy}>
                  <Play className={proxyBusy ? "animate-spin" : ""} />
                  启动
                </Button>
              )}
            </div>
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <TargetChip
              label="目标 OS"
              value={targetOs}
              options={TARGET_OS_OPTIONS}
              disabled={proxyRunning}
              onChange={(os) => handleTargetChange(os, targetArch)}
            />
            <TargetChip
              label="目标 Arch"
              value={targetArch}
              options={TARGET_ARCH_OPTIONS}
              disabled={proxyRunning}
              onChange={(arch) => handleTargetChange(targetOs, arch)}
            />
          </div>

          {proxyRunning ? (
            <div className="mt-3 rounded-[7px] border border-[#8fc9a3] bg-[#e5f6eb] px-3 py-3">
              <div className="flex items-center gap-2 text-xs font-bold text-[#1a6b3c]">
                <Server className="h-3.5 w-3.5" />
                已接管 — 127.0.0.1:{proxyStatus?.listen_port ?? 15800}
              </div>
              <ul className="mt-1.5 space-y-0.5 text-xs font-medium leading-5 text-[#2d6b46]">
                <li>Claude Code → 127.0.0.1:{proxyStatus?.listen_port ?? 15800}</li>
                <li className="break-all">
                  上游 → {upstreamUrl}（{upstreamMode}）
                </li>
                <li>出站 → 系统代理 / Clash TUN</li>
                <li>X-Stainless-OS → {targetOs}</li>
                <li>X-Stainless-Arch → {targetArch}</li>
                <li>context_management → null（自动剥离）</li>
              </ul>
            </div>
          ) : (
            <div className="mt-3 rounded-[7px] border border-dashed border-[#cbd8d1] px-3 py-2 text-xs font-semibold text-[#66796f]">
              未启动 — 代理上线后 probe 请求自动走代理
            </div>
          )}
        </section>

        <section className="px-5 py-4">
          <div className="mb-3 flex items-center justify-between gap-3">
            <h3 className="text-xs font-bold uppercase tracking-[0.1em] text-[#405149]">指纹历史</h3>
            <Badge variant="muted">
              <History />
              {history.length} 条
            </Badge>
          </div>
          <div className="space-y-2">
            {history.length ? (
              history.map((entry) => (
                <HistoryRow
                  key={entry.id}
                  entry={entry}
                  active={fingerprintsMatch(current, entry)}
                  disabled={busy}
                  onRestore={onRestore}
                  onDelete={onDelete}
                />
              ))
            ) : (
              <div className="rounded-[7px] border border-dashed border-[#cbd8d1] px-3 py-6 text-center text-xs font-semibold text-[#66796f]">
                暂无历史 — 切换指纹后自动保存
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

function TargetChip({
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly string[];
  disabled: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <div className="rounded-[7px] border border-[#dce7e1] bg-[#f8fbf9] px-3 py-2">
      <div className="text-xs font-semibold uppercase tracking-[0.08em] text-[#72847b]">{label}</div>
      <div className="mt-1.5 flex gap-1">
        {options.map((opt) => (
          <button
            key={opt}
            type="button"
            disabled={disabled}
            onClick={() => onChange(opt)}
            className={`rounded-[5px] px-2.5 py-1 text-xs font-semibold transition-colors ${
              value === opt ? "bg-[#17211d] text-white" : "bg-[#eef4f1] text-[#5b6f64] hover:bg-[#dce7e1]"
            } ${disabled ? "cursor-not-allowed opacity-50" : ""}`}
          >
            {opt}
          </button>
        ))}
      </div>
    </div>
  );
}

function HistoryRow({
  entry,
  active,
  disabled,
  onRestore,
  onDelete,
}: {
  entry: ClaudeFingerprintHistoryEntry;
  active: boolean;
  disabled: boolean;
  onRestore: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const restore = () => {
    if (window.confirm(`恢复到 ${shortDeviceId(entry.device_id)}？当前指纹会先保存到历史。`)) {
      onRestore(entry.id);
    }
  };
  const remove = () => {
    if (window.confirm("彻底删除这条历史指纹？")) {
      onDelete(entry.id);
    }
  };

  return (
    <div className="rounded-[7px] border border-[#dce7e1] bg-[#fbfdfc] px-3 py-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={active ? "success" : "muted"}>
              {active ? "当前" : fingerprintSourceLabel(entry.source)}
            </Badge>
            <span className="text-xs font-semibold text-[#66796f]" title={formatClock(entry.captured_at)}>
              {formatRelativeTime(entry.captured_at)}
            </span>
          </div>
          <div className="mt-1 text-xs font-semibold text-[#17211d]">
            {entry.stainless_os} / {entry.stainless_arch}
          </div>
          <div
            className="mono mt-1 break-all text-[11px] font-semibold text-[#5b6f64]"
            title={entry.device_id ?? undefined}
          >
            device_id: {shortDeviceId(entry.device_id)}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="icon"
            disabled={disabled || active}
            onClick={restore}
            title="恢复"
          >
            <RotateCcw />
          </Button>
          <Button type="button" variant="ghost" size="icon" disabled={disabled} onClick={remove} title="删除">
            <Trash2 />
          </Button>
        </div>
      </div>
    </div>
  );
}
