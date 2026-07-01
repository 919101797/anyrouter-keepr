import { useEffect, useState } from "react";
import type { ComponentType } from "react";
import {
  BrainCircuit,
  CircleAlert,
  Clock4,
  HardDrive,
  Hourglass,
  Moon,
  NotebookTabs,
  Power,
  Route,
  Shuffle,
  SlidersHorizontal,
  Terminal,
  Timer,
  X,
} from "lucide-react";
import { ActivityHeatmap } from "./components/ActivityHeatmap";
import { LiquidGlassBackdrop } from "./components/LiquidGlassBackdrop";
import { ProbeHistoryTable } from "./components/ProbeHistoryTable";
import { SettingsPanel } from "./components/SettingsPanel";
import { StatStrip } from "./components/StatStrip";
import { StatusHero } from "./components/StatusHero";
import { ThemePicker } from "./components/ThemePicker";
import { UpdatePanel, UpdateStatusButton } from "./components/UpdatePanel";
import { Badge } from "./components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { TooltipProvider } from "./components/ui/tooltip";
import { contextSizeLabel, effectiveModelValue, effortLabel } from "./lib/modelOptions";
import { formatTimeWindow } from "./lib/timeWindow";
import { useAppUpdater } from "./lib/useAppUpdater";
import { useAppTheme } from "./lib/useAppTheme";
import { useAppStore } from "./store/appStore";

export default function App() {
  const {
    profile,
    status,
    events,
    activity,
    claudeInstallation,
    claudeRuntimeConfig,
    claudeDetectionLogs,
    anchorTime,
    autostartEnabled,
    loading,
    busy,
    pendingAction,
    error,
    filter,
    load,
    refreshStatus,
    saveProfile,
    refreshClaudeInstallation,
    testClaudeInstallation,
    runProbeNow,
    startScheduler,
    pauseScheduler,
    setFilter,
    setAutostart,
  } = useAppStore();
  const visibleModel = profile?.model?.trim() || claudeRuntimeConfig?.default_model;
  const runtimeModel = effectiveModelValue(visibleModel, profile?.context_size) || "Claude Code 默认模型";
  const runtimeEffort = effortLabel(profile?.effort);
  const runtimeContext = contextSizeLabel(profile?.context_size);
  const promptPoolCount = profile?.prompt_pool?.filter((prompt) => prompt.trim()).length ?? 0;
  const { theme, setTheme } = useAppTheme();
  const updater = useAppUpdater();
  const [showStartupNotice, setShowStartupNotice] = useState(true);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void refreshStatus();
    }, 15_000);
    return () => window.clearInterval(timer);
  }, [refreshStatus]);

  return (
    <TooltipProvider delayDuration={180}>
      <main className="app-root h-screen overflow-auto overflow-x-hidden p-3 text-[#17211d] sm:p-5">
        <LiquidGlassBackdrop />
        <div className="app-shell mx-auto flex min-h-full max-w-[1500px] flex-col gap-4">
          <header className="app-header flex flex-col gap-3 rounded-[10px] border border-[#d2ded7] bg-white/[0.82] px-4 py-3 shadow-[0_14px_42px_rgba(36,55,47,0.08)] backdrop-blur md:flex-row md:items-center md:justify-between">
            <div className="flex min-w-0 items-center gap-3">
              <img
                src="/app-icon.png"
                alt="AnyRouter Keeper"
                className="h-10 w-10 shrink-0 rounded-[8px] object-cover shadow-[0_8px_22px_rgba(17,24,21,0.16)]"
              />
              <div className="min-w-0">
                <div className="truncate text-base font-bold tracking-normal">AnyRouter Keeper</div>
                <div className="mt-0.5 text-xs font-medium text-[#617369]">Claude Code activity console</div>
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <UpdateStatusButton updater={updater} />
              <ThemePicker theme={theme} onThemeChange={setTheme} />
              <Badge variant={claudeInstallation?.status === "ready" ? "success" : "warning"}>
                <Terminal />
                {claudeInstallation?.status === "ready" ? "CLI 已识别" : "CLI 待确认"}
              </Badge>
            </div>
          </header>

          {error ? (
            <div className="rounded-[8px] border border-[#f0a3af] bg-[#ffe0e5] px-4 py-3 text-sm font-medium text-[#8f1d31]">
              {error}
            </div>
          ) : null}

          <div className="app-content flex min-h-0 flex-1 flex-col gap-4">
            <div className="app-primary min-h-0 min-w-0 space-y-4 overflow-visible">
              <StatusHero
                profile={profile}
                claudeInstallation={claudeInstallation}
                claudeRuntimeConfig={claudeRuntimeConfig}
                status={status}
                busy={busy || loading}
                pendingAction={pendingAction}
                onStart={startScheduler}
                onPause={pauseScheduler}
                onProbe={runProbeNow}
                onSaveProfile={saveProfile}
              />
              <StatStrip status={status} events={events} />
              <ActivityHeatmap buckets={activity} anchorTime={anchorTime} />
              <ProbeHistoryTable events={events} filter={filter} onFilter={setFilter} />
            </div>

            <aside className="app-secondary min-h-0 min-w-0 overflow-visible">
              <Tabs defaultValue="settings">
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTrigger value="settings">设置</TabsTrigger>
                  <TabsTrigger value="runtime">运行参数</TabsTrigger>
                </TabsList>
                <TabsContent value="settings">
                  <SettingsPanel
                    key={profile ? JSON.stringify(profile) : "empty-profile"}
                    profile={profile}
                    claudeInstallation={claudeInstallation}
                    claudeRuntimeConfig={claudeRuntimeConfig}
                    claudeDetectionLogs={claudeDetectionLogs}
                    busy={busy || loading}
                    pendingAction={pendingAction}
                    autostartEnabled={autostartEnabled}
                    onSave={saveProfile}
                    onRefreshClaudeInstallation={refreshClaudeInstallation}
                    onTestClaudeInstallation={testClaudeInstallation}
                    onAutostartChange={setAutostart}
                  />
                </TabsContent>
                <TabsContent value="runtime">
                  <RuntimePanel
                    claudePath={
                      claudeInstallation?.effective_path || profile?.claude_binary_path || "PATH: claude"
                    }
                    endpoint={profile?.base_url?.trim() || "Claude Code / cc-switch 当前配置"}
                    model={runtimeModel}
                    effort={runtimeEffort}
                    context={runtimeContext}
                    interval={`${profile?.min_interval_seconds ?? 60}s - ${
                      profile?.max_interval_seconds ?? 120
                    }s`}
                    promptPool={promptPoolCount ? `${promptPoolCount} 条随机提示词` : "使用默认提示词"}
                    timeout={`${profile?.timeout_seconds ?? 90}s`}
                    window={formatTimeWindow(profile?.start_time, profile?.end_time)}
                    sleepPrevention={profile?.prevent_sleep ?? true}
                    autostart={autostartEnabled}
                  />
                </TabsContent>
              </Tabs>
            </aside>
          </div>
        </div>
        {showStartupNotice ? <StartupNotice onClose={() => setShowStartupNotice(false)} /> : null}
        <UpdatePanel updater={updater} />
      </main>
    </TooltipProvider>
  );
}

function StartupNotice({ onClose }: { onClose: () => void }) {
  return (
    <div className="startup-notice-overlay" role="presentation">
      <section
        className="startup-notice-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="startup-notice-title"
      >
        <button type="button" className="startup-notice-close" aria-label="关闭提示" onClick={onClose}>
          <X className="h-4 w-4" />
        </button>
        <div className="startup-notice-icon">
          <CircleAlert className="h-5 w-5" />
        </div>
        <div className="min-w-0">
          <h2 id="startup-notice-title" className="text-lg font-black tracking-normal text-[#121815]">
            保活提示
          </h2>
          <p className="mt-3 text-sm font-semibold leading-6 text-[#405149]">
            本应用的保活原理是定期对当前配置的 Claude Code 模型发起轻量调用，用来维持 AnyRouter
            通道活跃。每次探测都可能消耗额度或触发网关限流，不建议低额度用户长时间开启。
          </p>
          <div className="mt-5 flex justify-end">
            <button type="button" className="startup-notice-action" onClick={onClose}>
              知道了
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}

interface RuntimePanelProps {
  claudePath: string;
  endpoint: string;
  model: string;
  effort: string;
  context: string;
  interval: string;
  promptPool: string;
  timeout: string;
  window: string;
  sleepPrevention: boolean;
  autostart: boolean;
}

function RuntimePanel({
  claudePath,
  endpoint,
  model,
  effort,
  context,
  interval,
  promptPool,
  timeout,
  window,
  sleepPrevention,
  autostart,
}: RuntimePanelProps) {
  return (
    <div className="panel-ring overflow-hidden rounded-[8px] border border-[#d2ded7] bg-white">
      <div className="border-b border-[#dce7e1] px-5 py-4">
        <h2 className="text-sm font-bold text-[#121815]">运行参数</h2>
        <p className="mt-1 text-xs font-medium text-[#66796f]">当前守护进程使用的生效配置</p>
      </div>
      <div className="divide-y divide-[#edf3ef]">
        <Rule icon={Clock4} label="时间窗口" value={window} />
        <Rule
          icon={Moon}
          label="睡眠策略"
          value={sleepPrevention ? "守护运行时阻止系统睡眠" : "跟随系统睡眠设置"}
        />
        <Rule icon={Terminal} label="Claude CLI" value={claudePath} />
        <Rule icon={Route} label="Endpoint" value={endpoint} />
        <Rule icon={BrainCircuit} label="Model" value={model} />
        <Rule icon={SlidersHorizontal} label="推理强度" value={effort} />
        <Rule icon={NotebookTabs} label="上下文" value={context} />
        <Rule icon={Timer} label="随机间隔" value={interval} />
        <Rule icon={Shuffle} label="提示词池" value={promptPool} />
        <Rule icon={CircleAlert} label="错误策略" value="429 / 503 / 524 / reset / overloaded 继续抢" />
        <Rule icon={HardDrive} label="写盘策略" value="事件 buffer 批量 flush，不记录 tick" />
        <Rule icon={Power} label="开机自启" value={autostart ? "已开启" : "未开启"} />
        <Rule icon={Hourglass} label="超时" value={timeout} />
      </div>
    </div>
  );
}

function Rule({
  icon: Icon,
  label,
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="grid grid-cols-[32px_minmax(0,1fr)] items-start gap-3 px-5 py-3.5">
      <div className="runtime-rule-icon">
        <Icon className="h-4 w-4" />
      </div>
      <div className="min-w-0">
        <div className="text-xs font-semibold uppercase tracking-[0.08em] text-[#6a7d73]">{label}</div>
        <div className="mt-1 break-all text-sm font-semibold text-[#17211d]">{value}</div>
      </div>
    </div>
  );
}
