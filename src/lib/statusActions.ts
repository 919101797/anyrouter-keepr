export type StatusPendingAction =
  | "profile"
  | "claude"
  | "claude_path_test"
  | "probe"
  | "start"
  | "pause"
  | "autostart"
  | "fingerprint"
  | "fingerprint_refresh"
  | "proxy"
  | "proxy_target"
  | null;

export interface StatusActionViewInput {
  running: boolean;
  loading?: boolean;
  pendingAction?: StatusPendingAction;
}

export function getStatusActionView({
  running,
  loading = false,
  pendingAction = null,
}: StatusActionViewInput) {
  const guardPending = pendingAction === "start" || pendingAction === "pause";
  const probePending = pendingAction === "probe";
  const actionPending = Boolean(pendingAction) || loading;

  return {
    guardDisabled: actionPending,
    probeDisabled: actionPending,
    guardPending,
    probePending,
    guardLabel: guardPending
      ? pendingAction === "pause"
        ? "停止中"
        : "启动中"
      : running
        ? "暂停守护"
        : "开始守护",
    probeLabel: probePending ? "探测中" : "探测一次",
  };
}

export function getClaudePathActionView(pendingAction: StatusPendingAction = null) {
  const identifyPending = pendingAction === "claude_path_test";
  const savePending = pendingAction === "profile";
  const disabled = Boolean(pendingAction);

  return {
    identifyDisabled: disabled,
    saveDisabled: disabled,
    identifyPending,
    savePending,
    identifyLabel: "识别当前路径",
    saveLabel: savePending ? "保存中" : "保存路径",
  };
}
