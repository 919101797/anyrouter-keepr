import { useMemo, useState } from "react";
import type { ComponentProps, ComponentType, KeyboardEvent, ReactNode } from "react";
import { AlertTriangle, RefreshCw, Save, X } from "lucide-react";
import {
  CLAUDE_MODEL_OPTIONS,
  CONTEXT_SIZE_OPTIONS,
  CURRENT_MODEL_VALUE,
  CUSTOM_MODEL_VALUE,
  EFFORT_OPTIONS,
  defaultModelLabel,
  modelDisplayName,
  modelSelectValue,
  runtimeModelSelectValue,
} from "../lib/modelOptions";
import {
  DEFAULT_PROMPT_TAGS,
  MAX_PROMPT_TAGS,
  normalizePromptTags,
  promptTagsForUi,
} from "../lib/promptTags";
import { formatClock } from "../lib/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Switch } from "./ui/switch";
import type {
  ClaudeDetectionLog,
  ClaudeInstallation,
  ClaudeRuntimeConfig,
  ProfileInput,
  StoredProfile,
} from "../lib/types";

interface SettingsPanelProps {
  profile: StoredProfile | null;
  claudeInstallation: ClaudeInstallation | null;
  claudeRuntimeConfig: ClaudeRuntimeConfig | null;
  claudeDetectionLogs: ClaudeDetectionLog[];
  busy: boolean;
  autostartEnabled: boolean;
  onSave: (profile: ProfileInput) => void;
  onRefreshClaudeInstallation: () => void;
  onAutostartChange: (enabled: boolean) => void;
}

export function SettingsPanel({
  profile,
  claudeInstallation,
  claudeRuntimeConfig,
  claudeDetectionLogs,
  busy,
  autostartEnabled,
  onSave,
  onRefreshClaudeInstallation,
  onAutostartChange,
}: SettingsPanelProps) {
  const initial = useMemo<ProfileInput>(
    () =>
      profile
        ? { ...profile, token: "", prompt_pool: promptTagsForUi(profile.prompt_pool) }
        : {
            id: "default",
            name: "AnyRouter",
            claude_binary_path: "",
            base_url: "",
            token_kind: "ANTHROPIC_AUTH_TOKEN",
            token: "",
            model: "",
            effort: "low",
            context_size: "1m",
            prompt: "只回复 OK",
            prompt_pool: DEFAULT_PROMPT_TAGS,
            min_interval_seconds: 60,
            max_interval_seconds: 120,
            timeout_seconds: 90,
            start_time: "05:00",
            end_time: "24:00",
            enabled: false,
            stdout_summary_limit_bytes: 2048,
            stderr_summary_limit_bytes: 2048,
            event_flush_count: 5,
            event_flush_interval_seconds: 300,
            history_retention_days: 30,
            max_events_per_profile: 50000,
            max_database_size_mb: 50,
          },
    [profile],
  );
  const [draft, setDraft] = useState<ProfileInput>(initial);
  const [customModelMode, setCustomModelMode] = useState(
    () => modelSelectValue(initial.model) === CUSTOM_MODEL_VALUE,
  );
  const detectedDefaultModel = claudeRuntimeConfig?.default_model?.trim() || "";
  const selectedModel = customModelMode
    ? CUSTOM_MODEL_VALUE
    : runtimeModelSelectValue(draft.model, detectedDefaultModel);
  const customModelActive = selectedModel === CUSTOM_MODEL_VALUE;
  const promptTags = normalizePromptTags(draft.prompt_pool);
  const promptPoolCount = promptTags.length;
  const [newPromptTag, setNewPromptTag] = useState("");

  const update = <K extends keyof ProfileInput>(key: K, value: ProfileInput[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
  };

  const updateModelSelect = (value: string) => {
    if (value === CURRENT_MODEL_VALUE) {
      setCustomModelMode(false);
      update("model", "");
      return;
    }

    if (value === CUSTOM_MODEL_VALUE) {
      setCustomModelMode(true);
      if (!customModelActive && draft.model.trim()) {
        update("model", "");
      }
      return;
    }

    setCustomModelMode(false);
    update("model", value);
  };

  const updatePromptTags = (tags: string[]) => {
    update("prompt_pool", normalizePromptTags(tags));
  };

  const addPromptTag = () => {
    const value = newPromptTag.trim();
    if (!value || promptPoolCount >= MAX_PROMPT_TAGS) return;
    updatePromptTags([...promptTags, value]);
    setNewPromptTag("");
  };

  const removePromptTag = (tag: string) => {
    updatePromptTags(promptTags.filter((item) => item !== tag));
  };

  const handlePromptTagKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      addPromptTag();
    }
  };

  return (
    <div className="panel-ring overflow-hidden rounded-[8px] border border-[#d2ded7] bg-white">
      <div className="border-b border-[#dce7e1] px-5 py-4">
        <div>
          <h2 className="text-sm font-bold text-[#121815]">守护设置</h2>
          <p className="mt-0.5 text-xs font-medium text-[#66796f]">本机 Claude 配置优先</p>
        </div>
      </div>

      <div className="divide-y divide-[#edf3ef]">
        <Section title="Claude CLI">
          <div className="flex flex-col gap-3">
            <div className="flex items-start justify-between gap-3">
              <Badge variant={claudeStatusVariant(claudeInstallation?.status)}>
                {claudeStatusLabel(claudeInstallation?.status)}
              </Badge>
              <Button
                onClick={onRefreshClaudeInstallation}
                disabled={busy}
                variant="outline"
                size="sm"
                className="shrink-0"
              >
                <RefreshCw className={busy ? "animate-spin" : ""} />
                刷新
              </Button>
            </div>

            <div className="grid gap-2 sm:grid-cols-2 [&>*:last-child]:sm:col-span-2">
              <CliValue label="自动检测" value={claudeInstallation?.detected_path || "未检测到"} />
              <CliValue
                label="生效路径"
                value={
                  claudeInstallation?.effective_path ||
                  (draft.claude_binary_path.trim() ? "配置无效" : "PATH: claude")
                }
              />
              <CliValue label="版本" value={claudeInstallation?.version || "未知"} />
            </div>

            <Field label="Claude 可执行文件路径">
              <Input
                value={draft.claude_binary_path}
                placeholder="/usr/local/bin/claude，留空自动检测"
                onChange={(event) => update("claude_binary_path", event.target.value)}
              />
            </Field>

            {claudeInstallation?.error ? (
              <InlineNotice tone="warning" icon={AlertTriangle}>
                {claudeInstallation.error}
              </InlineNotice>
            ) : null}

            <DetectionLog logs={claudeDetectionLogs} />
          </div>
        </Section>

        <Section title="调用覆盖">
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="sm:col-span-2">
              <Field label="Endpoint">
                <Input
                  value={draft.base_url}
                  placeholder="留空使用 cc-switch / Claude Code 当前 endpoint"
                  onChange={(event) => update("base_url", event.target.value)}
                />
              </Field>
            </div>
            <div className="grid gap-3 sm:col-span-2 sm:grid-cols-[minmax(0,1fr)_120px_148px]">
              <Field label="Model">
                <Select value={selectedModel} onValueChange={updateModelSelect}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={CURRENT_MODEL_VALUE}>
                      {defaultModelLabel(detectedDefaultModel)}
                    </SelectItem>
                    {CLAUDE_MODEL_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {modelDisplayName(option.value)}
                      </SelectItem>
                    ))}
                    <SelectItem value={CUSTOM_MODEL_VALUE}>自定义模型</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="上下文">
                <Select
                  value={normalizedContextSize(draft.context_size)}
                  onValueChange={(value) => update("context_size", value)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {CONTEXT_SIZE_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
              <Field label="推理强度">
                <Select
                  value={normalizedEffort(draft.effort)}
                  onValueChange={(value) => update("effort", value)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {EFFORT_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            </div>
            {customModelActive ? (
              <div className="sm:col-span-2">
                <Field label="自定义模型">
                  <Input
                    value={draft.model}
                    placeholder="claude-opus-4-8[1m] / gateway-model-name"
                    onChange={(event) => update("model", event.target.value)}
                  />
                </Field>
              </div>
            ) : null}
            <div className="grid gap-3 sm:col-span-2">
              <Field label="默认提示词">
                <Input
                  value={draft.prompt}
                  placeholder="只回复 OK"
                  onChange={(event) => update("prompt", event.target.value)}
                />
              </Field>
              <Field label={`提示词池（${promptPoolCount} 条）`}>
                <PromptTagEditor
                  tags={promptTags}
                  value={newPromptTag}
                  onValueChange={setNewPromptTag}
                  onAdd={addPromptTag}
                  onRemove={removePromptTag}
                  onRestore={() => updatePromptTags(DEFAULT_PROMPT_TAGS)}
                  onKeyDown={handlePromptTagKeyDown}
                />
              </Field>
            </div>
            <Field label="Token 覆盖">
              <Input
                type="password"
                value={draft.token ?? ""}
                placeholder={profile?.has_token ? "已保存；留空保持不变" : "通常不需要填写"}
                onChange={(event) => update("token", event.target.value)}
              />
            </Field>
            <div className="sm:col-span-2">
              <Field label="Token 环境变量">
                <Select value={draft.token_kind} onValueChange={(value) => update("token_kind", value)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ANTHROPIC_AUTH_TOKEN">ANTHROPIC_AUTH_TOKEN</SelectItem>
                    <SelectItem value="ANTHROPIC_API_KEY">ANTHROPIC_API_KEY</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>
          </div>
        </Section>

        <Section title="时间与间隔">
          <div className="grid gap-3 sm:grid-cols-3">
            <Field label="最小间隔">
              <UnitInput
                value={draft.min_interval_seconds}
                unit="秒"
                onChange={(value) => update("min_interval_seconds", value)}
              />
            </Field>
            <Field label="最大间隔">
              <UnitInput
                value={draft.max_interval_seconds}
                unit="秒"
                onChange={(value) => update("max_interval_seconds", value)}
              />
            </Field>
            <Field label="超时">
              <UnitInput
                value={draft.timeout_seconds}
                unit="秒"
                onChange={(value) => update("timeout_seconds", value)}
              />
            </Field>
            <Field label="开始时间（HH:mm）">
              <Input
                value={draft.start_time}
                onChange={(event) => update("start_time", event.target.value)}
              />
            </Field>
            <Field label="结束时间（HH:mm）">
              <Input value={draft.end_time} onChange={(event) => update("end_time", event.target.value)} />
            </Field>
          </div>
        </Section>

        <Section title="存储与摘要">
          <div className="grid gap-3 sm:grid-cols-3">
            <Field label="Flush 条数">
              <UnitInput
                value={draft.event_flush_count}
                unit="条"
                onChange={(value) => update("event_flush_count", value)}
              />
            </Field>
            <Field label="Flush 间隔">
              <UnitInput
                value={draft.event_flush_interval_seconds}
                unit="秒"
                onChange={(value) => update("event_flush_interval_seconds", value)}
              />
            </Field>
            <Field label="DB 上限">
              <UnitInput
                value={draft.max_database_size_mb}
                unit="MB"
                onChange={(value) => update("max_database_size_mb", value)}
              />
            </Field>
            <Field label="stdout 摘要">
              <UnitInput
                value={draft.stdout_summary_limit_bytes}
                unit="字节"
                onChange={(value) => update("stdout_summary_limit_bytes", value)}
              />
            </Field>
            <Field label="stderr 摘要">
              <UnitInput
                value={draft.stderr_summary_limit_bytes}
                unit="字节"
                onChange={(value) => update("stderr_summary_limit_bytes", value)}
              />
            </Field>
            <Field label="保留天数">
              <UnitInput
                value={draft.history_retention_days}
                unit="天"
                onChange={(value) => update("history_retention_days", value)}
              />
            </Field>
          </div>
        </Section>

        <Section title="开关">
          <div className="grid gap-2">
            <ToggleRow
              label="保存启用状态"
              note="启动守护后自动同步"
              checked={draft.enabled}
              onCheckedChange={(value) => update("enabled", value)}
            />
            <ToggleRow
              label="开机自启"
              note="登录系统后启动"
              checked={autostartEnabled}
              disabled={busy}
              onCheckedChange={onAutostartChange}
            />
          </div>
        </Section>
      </div>

      <div className="flex justify-end border-t border-[#dce7e1] bg-[#f8fbf9] px-5 py-4">
        <Button onClick={() => onSave(draft)} disabled={busy}>
          <Save />
          保存设置
        </Button>
      </div>
    </div>
  );
}

function normalizedEffort(value: string) {
  return EFFORT_OPTIONS.some((option) => option.value === value) ? value : "low";
}

function normalizedContextSize(value: string) {
  return CONTEXT_SIZE_OPTIONS.some((option) => option.value === value) ? value : "1m";
}

function PromptTagEditor({
  tags,
  value,
  onValueChange,
  onAdd,
  onRemove,
  onRestore,
  onKeyDown,
}: {
  tags: string[];
  value: string;
  onValueChange: (value: string) => void;
  onAdd: () => void;
  onRemove: (tag: string) => void;
  onRestore: () => void;
  onKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
}) {
  return (
    <div className="prompt-pool-editor">
      <div className="prompt-pool-list" role="list">
        {tags.map((tag) => (
          <div key={tag} className="prompt-pool-item" role="listitem">
            <span className="prompt-pool-text">{tag}</span>
            <button
              type="button"
              className="prompt-pool-delete"
              aria-label={`删除 ${tag}`}
              onClick={() => onRemove(tag)}
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        ))}
      </div>
      <div className="prompt-pool-actions">
        <Input
          value={value}
          placeholder="新增轻量提示词"
          maxLength={80}
          className="prompt-pool-input"
          onChange={(event) => onValueChange(event.target.value)}
          onKeyDown={onKeyDown}
        />
        <Button type="button" variant="secondary" onClick={onAdd} disabled={!value.trim()}>
          添加
        </Button>
        <Button type="button" variant="outline" className="px-3" onClick={onRestore}>
          恢复默认
        </Button>
      </div>
    </div>
  );
}

function UnitInput({
  value,
  unit,
  onChange,
}: {
  value: number;
  unit: string;
  onChange: (value: number) => void;
}) {
  return (
    <div className="relative">
      <Input
        type="number"
        value={value}
        className="pr-14"
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <span className="unit-input-suffix">{unit}</span>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="px-5 py-4">
      <h3 className="mb-3 text-xs font-bold uppercase tracking-[0.1em] text-[#405149]">{title}</h3>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0 space-y-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function CliValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-[7px] border border-[#dce7e1] bg-[#f8fbf9] px-3 py-2">
      <div className="text-xs font-semibold uppercase tracking-[0.08em] text-[#72847b]">{label}</div>
      <div className="mono mt-1 min-h-5 break-all text-xs font-semibold text-[#17211d]">{value}</div>
    </div>
  );
}

function DetectionLog({ logs }: { logs: ClaudeDetectionLog[] }) {
  return (
    <div className="rounded-[7px] border border-[#dce7e1] bg-[#f8fbf9]">
      <div className="border-b border-[#e4ece8] px-3 py-2 text-xs font-bold uppercase tracking-[0.08em] text-[#617369]">
        Claude 路径更新日志
      </div>
      <div className="divide-y divide-[#e4ece8]">
        {logs.length ? (
          logs.slice(0, 5).map((log) => (
            <div key={log.id} className="px-3 py-2.5">
              <div className="min-w-0">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-xs font-semibold text-[#66796f]">{formatClock(log.checked_at)}</span>
                  <Badge variant={claudeStatusVariant(log.status)}>{claudeStatusLabel(log.status)}</Badge>
                </div>
                <div className="mono mt-1 break-all text-xs font-semibold text-[#17211d]">
                  {log.effective_path || log.configured_path || "PATH: claude"}
                </div>
                <div className="mt-1 break-all text-xs font-medium text-[#66796f]">
                  {log.version || log.error || sourceLabel(log.source)}
                </div>
              </div>
            </div>
          ))
        ) : (
          <div className="px-3 py-3 text-xs font-medium text-[#66796f]">暂无手动刷新或路径变更记录。</div>
        )}
      </div>
    </div>
  );
}

function ToggleRow({
  label,
  note,
  checked,
  disabled,
  onCheckedChange,
}: {
  label: string;
  note: string;
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-[7px] border border-[#dce7e1] bg-[#f8fbf9] px-3 py-2.5">
      <div className="min-w-0">
        <div className="text-sm font-bold text-[#17211d]">{label}</div>
        <div className="mt-0.5 text-xs font-medium text-[#66796f]">{note}</div>
      </div>
      <Switch checked={checked} disabled={disabled} onCheckedChange={onCheckedChange} />
    </div>
  );
}

function InlineNotice({
  tone,
  icon: Icon,
  children,
}: {
  tone: "warning";
  icon: ComponentType<{ className?: string }>;
  children: ReactNode;
}) {
  const className =
    tone === "warning"
      ? "border-[#e7c75e] bg-[#fff7df] text-[#85610e]"
      : "border-[#dce7e1] bg-[#f8fbf9] text-[#52645b]";

  return (
    <div
      className={`flex items-start gap-2 rounded-[7px] border px-3 py-2 text-xs font-semibold ${className}`}
    >
      <Icon className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      <span className="break-all">{children}</span>
    </div>
  );
}

function claudeStatusLabel(status?: string | null) {
  switch (status) {
    case "ready":
      return "已识别";
    case "not_found":
      return "未找到";
    case "invalid":
      return "不可用";
    default:
      return "待检测";
  }
}

function claudeStatusVariant(status?: string | null): ComponentProps<typeof Badge>["variant"] {
  switch (status) {
    case "ready":
      return "success";
    case "not_found":
      return "warning";
    case "invalid":
      return "danger";
    default:
      return "muted";
  }
}

function sourceLabel(source: string) {
  return source === "manual" ? "手动配置" : "PATH 自动检测";
}
