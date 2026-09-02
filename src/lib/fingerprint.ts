export interface ClaudeFingerprint {
  checked_at: string;
  claude_state_path: string;
  stainless_os: string;
  stainless_arch: string;
  device_id?: string | null;
  device_id_status: string;
  session_id_status: string;
  risk_label: string;
}

export interface ClaudeFingerprintHistoryEntry {
  id: string;
  captured_at: string;
  source: string;
  claude_state_path: string;
  stainless_os: string;
  stainless_arch: string;
  device_id?: string | null;
  device_id_status: string;
  session_id_status: string;
  risk_label: string;
}

export interface ClaudeFingerprintSnapshot {
  current?: ClaudeFingerprint | null;
  history: ClaudeFingerprintHistoryEntry[];
  error?: string | null;
}

export interface ProxyStatus {
  running: boolean;
  listen_port: number;
  target_os: string;
  target_arch: string;
  upstream_url: string;
  dynamic_upstream?: boolean;
  error: string | null;
}

export const FINGERPRINT_TARGETS = [
  { os: "Windows", arch: "x64" },
  { os: "Windows", arch: "arm64" },
  { os: "Linux", arch: "x64" },
  { os: "Linux", arch: "arm64" },
  { os: "MacOS", arch: "x64" },
  { os: "MacOS", arch: "arm64" },
] as const;

export function shortDeviceId(value?: string | null) {
  if (!value) return "未发现";
  if (value.length <= 20) return value;
  return `${value.slice(0, 8)}...${value.slice(-8)}`;
}

export function chooseNextFingerprintTarget(
  currentOs = "Windows",
  currentArch = "x64",
  random = Math.random,
) {
  const candidates = FINGERPRINT_TARGETS.filter(
    (target) => target.os !== currentOs || target.arch !== currentArch,
  );
  const index = Math.floor(random() * candidates.length);
  return candidates[Math.min(index, candidates.length - 1)] ?? FINGERPRINT_TARGETS[0];
}

export function fingerprintItemDetail(
  itemKey: string,
  fingerprint: ClaudeFingerprint,
  proxyStatus: ProxyStatus | null | undefined,
) {
  const proxyRunning = proxyStatus?.running ?? false;
  const targetOs = proxyStatus?.target_os?.trim();
  const targetArch = proxyStatus?.target_arch?.trim();

  switch (itemKey) {
    case "x-stainless-os":
      if (proxyRunning && targetOs) return `代理生效: ${targetOs}（本机: ${fingerprint.stainless_os}）`;
      if (targetOs && targetOs !== fingerprint.stainless_os)
        return `目标: ${targetOs}（未启动，本机: ${fingerprint.stainless_os}）`;
      return fingerprint.stainless_os;
    case "x-stainless-arch":
      if (proxyRunning && targetArch) return `代理生效: ${targetArch}（本机: ${fingerprint.stainless_arch}）`;
      if (targetArch && targetArch !== fingerprint.stainless_arch) {
        return `目标: ${targetArch}（未启动，本机: ${fingerprint.stainless_arch}）`;
      }
      return fingerprint.stainless_arch;
    case "device_id":
      return shortDeviceId(fingerprint.device_id);
    case "session_id":
      return "Claude Code 运行时自动维护一致性";
    case "context_management":
      return proxyRunning ? "代理自动剥离为 null" : "未启动，当前由 Claude Code 自行控制";
    default:
      return "—";
  }
}

export function fingerprintSourceLabel(source: string) {
  const labels: Record<string, string> = {
    captured_before_regenerate: "切换前保存",
    captured_before_restore: "恢复前保存",
    generated: "重新生成",
    restored: "已恢复",
  };
  return labels[source] ?? source;
}

export function fingerprintsMatch(
  current: ClaudeFingerprint | null | undefined,
  history: ClaudeFingerprintHistoryEntry,
) {
  return Boolean(
    current?.device_id &&
    history.device_id &&
    current.device_id === history.device_id &&
    current.stainless_os === history.stainless_os &&
    current.stainless_arch === history.stainless_arch,
  );
}

export interface FingerprintMatrixItem {
  key: string;
  label: string;
  location: string;
  impact: "高置信" | "中低置信" | "已排除";
  note: string;
}

export const FINGERPRINT_MATRIX: FingerprintMatrixItem[] = [
  {
    key: "x-stainless-os",
    label: "X-Stainless-OS",
    location: "HTTP Header",
    impact: "高置信",
    note: "MacOS 触发 429，Windows 正常通过",
  },
  {
    key: "x-stainless-arch",
    label: "X-Stainless-Arch",
    location: "HTTP Header",
    impact: "中低置信",
    note: "单独 arm64 不影响（仍 200）；与 OS 组合被排查过但非根因",
  },
  {
    key: "device_id",
    label: "meta.user_id.device_id",
    location: "JSON Body",
    impact: "高置信",
    note: "非任意 64 hex 可通过，Mac/随机 device_id 触发 429",
  },
  {
    key: "session_id",
    label: "X-Claude-Code-Session-Id ↔ session_id",
    location: "Header ↔ Body",
    impact: "高置信",
    note: "header 与 body 中的 session_id 必须一致，否则 429",
  },
  {
    key: "context_management",
    label: "context_management",
    location: "JSON Body",
    impact: "高置信",
    note: "{} 或含 edits 触发 429，缺失/null 可过",
  },
];

export interface FingerprintDiagnosis {
  level: "fail" | "pass" | "unknown";
  summary: string;
  items: { item: FingerprintMatrixItem; status: "risky" | "safe" | "neutral"; detail: string }[];
  suggestion: string | null;
}

export function diagnoseFingerprint(
  fingerprint: ClaudeFingerprint | null | undefined,
  proxyRunning: boolean,
  targetOs: string,
): FingerprintDiagnosis {
  if (!fingerprint) {
    return {
      level: "unknown",
      summary: "无法读取指纹",
      items: [],
      suggestion: "请确认 Claude Code 已安装并至少运行过一次",
    };
  }

  if (!fingerprint.device_id) {
    return {
      level: "unknown",
      summary: "device_id 缺失",
      items: [],
      suggestion: "请先运行一次 Claude Code 以生成 device_id",
    };
  }

  const effectiveOs = proxyRunning ? targetOs : fingerprint.stainless_os;
  const arch = fingerprint.stainless_arch;
  const hasDeviceId = fingerprint.device_id_status === "present";

  const items: FingerprintDiagnosis["items"] = [
    {
      item: FINGERPRINT_MATRIX[0],
      status: effectiveOs === "MacOS" ? "risky" : effectiveOs === "Windows" ? "safe" : "neutral",
      detail: proxyRunning
        ? `代理伪装为 ${effectiveOs}（原始: ${fingerprint.stainless_os}）`
        : `当前 ${effectiveOs}`,
    },
    {
      item: FINGERPRINT_MATRIX[1],
      status: "neutral",
      detail: `当前 ${arch}（非独立影响项）`,
    },
    {
      item: FINGERPRINT_MATRIX[2],
      status: hasDeviceId ? "neutral" : "risky",
      detail: hasDeviceId ? `${shortDeviceId(fingerprint.device_id)}` : "缺失",
    },
    {
      item: FINGERPRINT_MATRIX[3],
      status: "neutral",
      detail: "Claude Code 运行时自动维护一致性（app probe 使用 --no-session-persistence）",
    },
    {
      item: FINGERPRINT_MATRIX[4],
      status: proxyRunning ? "safe" : "neutral",
      detail: proxyRunning ? "代理自动剥离非 null 的 context_management" : "未代理时由 Claude Code 自行控制",
    },
  ];

  const hasRisky = items.filter((i) => i.status === "risky").length;
  const effectiveItems = items.filter((i) => i.item.impact === "高置信" && i.status === "risky");

  if (hasRisky > 0 && !proxyRunning) {
    return {
      level: "fail",
      summary: `${effectiveItems.length} 项高置信指纹处于风险状态`,
      items,
      suggestion: "启动指纹代理可修复 X-Stainless-OS 和 context_management；device_id 可一键重新生成",
    };
  }

  if (proxyRunning && hasRisky === 0) {
    return {
      level: "pass",
      summary: "代理运行中，所有高置信项处于安全状态",
      items,
      suggestion: null,
    };
  }

  if (hasRisky === 0) {
    return {
      level: "pass",
      summary: "所有高置信项处于安全状态",
      items,
      suggestion: null,
    };
  }

  return {
    level: "unknown",
    summary: "部分指纹项状态不确定",
    items,
    suggestion: "启动指纹代理并重新生成 device_id 确保安全",
  };
}

export function fingerprintRiskLevel(
  fingerprint: ClaudeFingerprint | null | undefined,
): "risky" | "safe" | "unknown" {
  const diagnosis = diagnoseFingerprint(fingerprint, false, "");
  if (diagnosis.level === "fail") return "risky";
  if (diagnosis.level === "pass") return "safe";
  return "unknown";
}
