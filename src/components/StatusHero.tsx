import { Check, Clock3, Pause, Play, RotateCw, SatelliteDish, Terminal, X, Zap } from "lucide-react";
import { useState } from "react";
import type { ComponentType, ReactNode } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { StatusPill } from "./StatusPill";
import {
  CLAUDE_MODEL_OPTIONS,
  CONTEXT_SIZE_OPTIONS,
  CURRENT_MODEL_VALUE,
  CUSTOM_MODEL_VALUE,
  EFFORT_OPTIONS,
  defaultModelLabel,
  modelDisplayName,
  runtimeModelSelectValue,
} from "../lib/modelOptions";
import { formatClock, formatRelativeTime } from "../lib/utils";
import type { AppStatus, ClaudeInstallation, ProfileInput, StoredProfile } from "../lib/types";
import type { ClaudeRuntimeConfig } from "../lib/types";

interface StatusHeroProps {
  profile: StoredProfile | null;
  claudeInstallation: ClaudeInstallation | null;
  claudeRuntimeConfig: ClaudeRuntimeConfig | null;
  status: AppStatus | null;
  busy: boolean;
  onStart: () => void;
  onPause: () => void;
  onProbe: () => void;
  onSaveProfile: (profile: ProfileInput) => Promise<void>;
}

const statusTone = {
  connected: {
    rail: "bg-[#55e08f]",
    accent: "bg-[#cfff4e] text-[#121815]",
    line: "border-[#cfff4e]/55",
    glow: "shadow-[0_0_0_1px_rgba(207,255,78,0.18),0_24px_80px_rgba(102,255,169,0.12)]",
  },
  racing: {
    rail: "bg-[#ffd45d]",
    accent: "bg-[#ffd45d] text-[#121815]",
    line: "border-[#ffd45d]/55",
    glow: "shadow-[0_0_0_1px_rgba(255,212,93,0.2),0_24px_80px_rgba(255,212,93,0.09)]",
  },
  config_error: {
    rail: "bg-[#ff6077]",
    accent: "bg-[#ff6077] text-white",
    line: "border-[#ff6077]/55",
    glow: "shadow-[0_0_0_1px_rgba(255,96,119,0.22),0_24px_80px_rgba(255,96,119,0.1)]",
  },
  neutral: {
    rail: "bg-[#54e7db]",
    accent: "bg-[#54e7db] text-[#121815]",
    line: "border-[#54e7db]/55",
    glow: "shadow-[0_0_0_1px_rgba(84,231,219,0.18),0_24px_80px_rgba(84,231,219,0.1)]",
  },
};

export function StatusHero({
  profile,
  claudeInstallation,
  claudeRuntimeConfig,
  status,
  busy,
  onStart,
  onPause,
  onProbe,
  onSaveProfile,
}: StatusHeroProps) {
  const [customModelOpen, setCustomModelOpen] = useState(false);
  const [customModelDraft, setCustomModelDraft] = useState("");
  const running = Boolean(status?.running);
  const state = status?.current_state ?? "paused";
  const endpoint = profile?.base_url?.trim() || "Claude Code / cc-switch 当前配置";
  const cliPath = claudeInstallation?.effective_path || profile?.claude_binary_path?.trim() || "PATH: claude";
  const cliVersion = claudeInstallation?.version || "未检测版本";
  const tone =
    state === "connected"
      ? statusTone.connected
      : state === "racing" || state === "probing"
        ? statusTone.racing
        : state === "config_error"
          ? statusTone.config_error
          : statusTone.neutral;
  const headline =
    state === "connected"
      ? "链路在线"
      : state === "racing"
        ? "正在抢活性"
        : state === "config_error"
          ? "配置待修"
          : state === "sleeping"
            ? "窗口外休眠"
            : state === "probing"
              ? "正在探测"
              : running
                ? "守护运行中"
                : "守护已暂停";
  const detectedDefaultModel = claudeRuntimeConfig?.default_model?.trim() || "";
  const profileModel = profile?.model?.trim() || "";
  const modelChoice = runtimeModelSelectValue(profileModel, detectedDefaultModel);
  const modelSelectLabel = modelChoice === CUSTOM_MODEL_VALUE ? profileModel || "自定义模型" : "自定义模型";
  const contextChoice = normalizedContext(profile?.context_size);
  const effortChoice = normalizedEffort(profile?.effort);

  const savePatch = async (patch: Partial<Pick<ProfileInput, "model" | "effort" | "context_size">>) => {
    if (!profile) return;

    const { has_token, ...input } = profile;
    void has_token;
    await onSaveProfile({ ...input, ...patch });
  };

  const changeModel = (value: string) => {
    if (value === CUSTOM_MODEL_VALUE) {
      setCustomModelDraft(modelChoice === CUSTOM_MODEL_VALUE ? (profile?.model ?? "") : "");
      setCustomModelOpen(true);
      return;
    }

    setCustomModelOpen(false);
    void savePatch({ model: value === CURRENT_MODEL_VALUE ? "" : value });
  };

  const saveCustomModel = () => {
    const model = customModelDraft.trim();
    if (!model) return;
    setCustomModelOpen(false);
    void savePatch({ model });
  };

  return (
    <section className={`status-hero panel-ring relative overflow-hidden rounded-[10px] border ${tone.glow}`}>
      <div className="status-hero-grid absolute inset-0" />
      <div className="status-hero-lines absolute inset-y-0 right-0 w-2/5" />
      <div className={`absolute inset-y-0 left-0 w-1.5 ${tone.rail}`} />

      <div className="relative grid gap-6 p-5 sm:p-7 lg:grid-cols-[minmax(0,1fr)_260px]">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-3">
            <div
              className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-[8px] ${tone.accent}`}
            >
              <SatelliteDish className="h-6 w-6" />
            </div>
            <div className="min-w-0">
              <div className="status-hero-eyebrow truncate text-sm font-semibold">{endpoint}</div>
              <div className="mt-1 flex flex-wrap items-center gap-2">
                <div className="status-hero-select-shell flex min-w-0 max-w-full flex-wrap items-center gap-1 rounded-[7px] border p-1 sm:flex-nowrap">
                  <InlineSelect
                    value={modelChoice}
                    disabled={busy || !profile}
                    className="w-[clamp(156px,18vw,220px)]"
                    onValueChange={changeModel}
                  >
                    <SelectItem value={CURRENT_MODEL_VALUE}>
                      {defaultModelLabel(detectedDefaultModel)}
                    </SelectItem>
                    {CLAUDE_MODEL_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {modelDisplayName(option.value)}
                      </SelectItem>
                    ))}
                    <SelectItem value={CUSTOM_MODEL_VALUE}>{modelSelectLabel}</SelectItem>
                  </InlineSelect>
                  <InlineSelect
                    value={contextChoice}
                    disabled={busy || !profile}
                    className="w-[70px]"
                    onValueChange={(value) => void savePatch({ context_size: value })}
                  >
                    {CONTEXT_SIZE_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </InlineSelect>
                  <InlineSelect
                    value={effortChoice}
                    disabled={busy || !profile}
                    className="w-[88px]"
                    onValueChange={(value) => void savePatch({ effort: value })}
                  >
                    {EFFORT_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </InlineSelect>
                </div>
                <StatusPill status={state} />
              </div>
              {customModelOpen ? (
                <div className="mt-2 flex max-w-xl items-center gap-2">
                  <Input
                    autoFocus
                    value={customModelDraft}
                    placeholder="claude-opus-4-8 / gateway-model-name"
                    className="status-hero-custom-input h-8 text-xs font-semibold"
                    onChange={(event) => setCustomModelDraft(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") saveCustomModel();
                      if (event.key === "Escape") setCustomModelOpen(false);
                    }}
                  />
                  <Button
                    onClick={saveCustomModel}
                    disabled={busy || !customModelDraft.trim()}
                    size="icon"
                    className="h-8 w-8 shrink-0 bg-[#cfff4e] text-[#121815] hover:bg-[#dfff78]"
                    aria-label="保存自定义模型"
                  >
                    <Check />
                  </Button>
                  <Button
                    onClick={() => setCustomModelOpen(false)}
                    disabled={busy}
                    variant="outline"
                    size="icon"
                    className="status-hero-outline-icon h-8 w-8 shrink-0"
                    aria-label="取消自定义模型"
                  >
                    <X />
                  </Button>
                </div>
              ) : null}
            </div>
          </div>

          <h1 className="status-hero-title mt-7 text-4xl font-black tracking-normal md:text-5xl">
            {headline}
          </h1>
          <p className="status-hero-copy mt-3 max-w-2xl text-sm font-medium leading-6">
            05:00 - 24:00 随机探测，遇到队列错误继续保持活性。
          </p>

          <div className="mt-6 grid gap-2 sm:grid-cols-2 2xl:grid-cols-4">
            <HeroMetric label="最近成功" value={formatRelativeTime(status?.last_success_at)} />
            <HeroMetric label="最近请求" value={formatClock(status?.last_event?.started_at)} />
            <HeroMetric label="下一次" value={formatClock(status?.next_probe_at)} />
            <HeroMetric label="CLI" value={cliPath} mono />
          </div>
        </div>

        <div className="status-hero-side-panel flex min-w-0 flex-col justify-between gap-4 rounded-[8px] border p-4">
          <div className="space-y-3">
            <SideFact icon={Terminal} label="Claude" value={cliVersion} />
            <SideFact icon={Clock3} label="状态窗口" value={status?.in_window ? "窗口内" : "窗口外"} />
          </div>
          <div className="grid gap-2">
            <Button
              onClick={running ? onPause : onStart}
              disabled={busy}
              variant="secondary"
              className={
                running
                  ? "status-hero-pause-button"
                  : "border-transparent bg-[#cfff4e] text-[#121815] hover:bg-[#dfff78]"
              }
            >
              {running ? <Pause /> : <Play />}
              {running ? "暂停守护" : "开始守护"}
            </Button>
            <Button onClick={onProbe} disabled={busy} variant="outline" className="status-hero-probe-button">
              {busy ? <RotateCw className="animate-spin" /> : <Zap />}
              探测一次
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}

function InlineSelect({
  value,
  disabled,
  className,
  children,
  onValueChange,
}: {
  value: string;
  disabled: boolean;
  className?: string;
  children: ReactNode;
  onValueChange: (value: string) => void;
}) {
  return (
    <Select value={value} disabled={disabled} onValueChange={onValueChange}>
      <SelectTrigger
        className={`status-hero-inline-select mono h-8 px-2 py-1 text-xs font-semibold shadow-none [&>span]:truncate ${className ?? ""}`}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>{children}</SelectContent>
    </Select>
  );
}

function normalizedEffort(value?: string | null) {
  const candidate = value ?? "";
  return EFFORT_OPTIONS.some((option) => option.value === candidate) ? candidate : "low";
}

function normalizedContext(value?: string | null) {
  const candidate = value ?? "";
  return CONTEXT_SIZE_OPTIONS.some((option) => option.value === candidate) ? candidate : "1m";
}

function HeroMetric({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="status-hero-metric min-w-0 rounded-[7px] border px-3 py-2.5">
      <div className="status-hero-metric-label text-xs font-semibold uppercase tracking-[0.08em]">
        {label}
      </div>
      <div className={`status-hero-metric-value mt-1 truncate text-sm font-bold ${mono ? "mono" : ""}`}>
        {value}
      </div>
    </div>
  );
}

function SideFact({
  icon: Icon,
  label,
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="grid grid-cols-[32px_minmax(0,1fr)] items-center gap-3">
      <div className="status-hero-side-icon flex h-8 w-8 items-center justify-center rounded-[6px] border text-[#cfff4e]">
        <Icon className="h-4 w-4" />
      </div>
      <div className="min-w-0">
        <div className="status-hero-metric-label text-xs font-semibold uppercase tracking-[0.08em]">
          {label}
        </div>
        <div className="status-hero-side-value mt-0.5 truncate text-sm font-bold">{value}</div>
      </div>
    </div>
  );
}
