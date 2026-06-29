export const CURRENT_MODEL_VALUE = "__current_claude_code__";
export const CUSTOM_MODEL_VALUE = "__custom_model__";

export const CLAUDE_MODEL_OPTIONS = [
  { value: "claude-fable-5", label: "Claude Fable 5" },
  { value: "claude-opus-4-8", label: "Claude Opus 4.8" },
  { value: "claude-sonnet-4-6", label: "Claude Sonnet 4.6" },
  { value: "claude-haiku-4-5", label: "Claude Haiku 4.5" },
  { value: "claude-mythos-5", label: "Claude Mythos 5" },
] as const;

export const EFFORT_OPTIONS = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "XHigh" },
  { value: "max", label: "Max" },
] as const;

export const CONTEXT_SIZE_OPTIONS = [
  { value: "1m", label: "1M" },
  { value: "native", label: "原生" },
] as const;

export function modelSelectValue(model: string) {
  const value = canonicalModelValue(model);
  if (!value) return CURRENT_MODEL_VALUE;
  if (CLAUDE_MODEL_OPTIONS.some((option) => option.value === value)) return value;
  return CUSTOM_MODEL_VALUE;
}

export function runtimeModelSelectValue(
  configuredModel?: string | null,
  detectedDefaultModel?: string | null,
) {
  const configured = configuredModel?.trim() ?? "";
  if (configured) return modelSelectValue(configured);

  const detected = canonicalModelValue(detectedDefaultModel);
  if (!detected) return CUSTOM_MODEL_VALUE;

  return CLAUDE_MODEL_OPTIONS.some((option) => option.value === detected) ? detected : CUSTOM_MODEL_VALUE;
}

export function canonicalModelValue(model?: string | null) {
  const value = model?.trim().replace(/\[[^\]]+\]$/, "") ?? "";
  if (!value) return "";

  const exact = CLAUDE_MODEL_OPTIONS.find((option) => option.value === value);
  if (exact) return exact.value;

  const dated = value.replace(/-\d{8}$/, "");
  const datedMatch = CLAUDE_MODEL_OPTIONS.find((option) => option.value === dated);
  if (datedMatch) return datedMatch.value;

  return value;
}

export function modelDisplayName(model?: string | null) {
  const value = canonicalModelValue(model);
  if (!value) return "";

  return CLAUDE_MODEL_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

export function defaultModelLabel(model?: string | null) {
  const displayName = modelDisplayName(model);
  return displayName ? `默认：${displayName}` : "未识别模型";
}

export function effectiveModelValue(model?: string | null, contextSize?: string | null) {
  const value = model?.trim() ?? "";
  if (!value) return "";
  if (shouldAppendContextSuffix(value, contextSize ?? "")) return `${value}[1m]`;
  return value;
}

export function contextSizeLabel(contextSize?: string | null) {
  return contextSize?.trim().toLowerCase() === "1m" ? "1M" : "原生";
}

export function effortLabel(effort?: string | null) {
  const value = effort?.trim() || "auto";
  return value === "xhigh" ? "XHigh" : value.charAt(0).toUpperCase() + value.slice(1);
}

function shouldAppendContextSuffix(model: string, contextSize: string) {
  return contextSize.trim().toLowerCase() === "1m" && !model.includes("[") && !isKnownNon1mModel(model);
}

function isKnownNon1mModel(model: string) {
  const value = model.trim().toLowerCase();
  return value === "haiku" || value === "opusplan" || value.startsWith("claude-haiku");
}
