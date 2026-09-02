import type { UpstreamModel } from "./types";

export const CUSTOM_MODEL_VALUE = "__custom_model__";
export const EMPTY_MODEL_OPTIONS_VALUE = "__empty_upstream_models__";

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

export function modelSelectValue(model: string, options: UpstreamModel[]) {
  const value = canonicalModelValue(model);
  if (options.some((option) => option.id === value)) return value;
  return CUSTOM_MODEL_VALUE;
}

export function runtimeModelSelectValue(
  configuredModel: string | null | undefined,
  detectedDefaultModel: string | null | undefined,
  options: UpstreamModel[],
) {
  const model = configuredModel?.trim() || detectedDefaultModel?.trim() || "";
  return modelSelectValue(model, options);
}

export function canonicalModelValue(model?: string | null) {
  return model?.trim().replace(/\[[^\]]+\]$/, "") ?? "";
}

export function modelDisplayName(model: string | null | undefined, options: UpstreamModel[]) {
  const value = canonicalModelValue(model);
  if (!value) return "";
  return options.find((option) => option.id === value)?.display_name ?? value;
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
