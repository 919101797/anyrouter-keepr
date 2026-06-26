import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatRelativeTime(value?: string | null) {
  if (!value) return "从未";
  const time = new Date(value).getTime();
  const diff = Date.now() - time;
  if (!Number.isFinite(diff)) return "未知";
  if (diff < 30_000) return "刚刚";
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  return `${days} 天前`;
}

export function formatDuration(ms?: number | null) {
  if (ms == null) return "-";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export function formatLongDuration(ms?: number | null) {
  if (ms == null) return "-";
  if (ms < 60_000) return formatDuration(ms);
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 60) return `${minutes} 分钟`;
  const hours = minutes / 60;
  if (hours < 48) return `${hours.toFixed(1)} 小时`;
  const days = hours / 24;
  return `${days.toFixed(1)} 天`;
}

export function formatClock(value?: string | null) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function statusLabel(status?: string | null) {
  switch (status) {
    case "success":
      return "已联通";
    case "queue_miss":
      return "抢占中";
    case "timeout":
      return "超时";
    case "config_error":
      return "配置错误";
    case "unknown":
      return "未知";
    default:
      return "暂无";
  }
}
