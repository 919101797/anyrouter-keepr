import { AlertTriangle, CheckCircle2, Clock3, RadioTower, XCircle } from "lucide-react";
import { Badge } from "./ui/badge";
import type { ProbeStatus } from "../lib/types";

export function StatusPill({ status }: { status?: ProbeStatus | string | null }) {
  if (status === "success" || status === "connected") {
    return (
      <Badge variant="success" className="gap-1">
        <CheckCircle2 className="h-3.5 w-3.5" />
        已联通
      </Badge>
    );
  }
  if (status === "queue_miss" || status === "racing") {
    return (
      <Badge variant="warning" className="gap-1">
        <RadioTower className="h-3.5 w-3.5" />
        抢占中
      </Badge>
    );
  }
  if (status === "timeout") {
    return (
      <Badge variant="warning" className="gap-1">
        <Clock3 className="h-3.5 w-3.5" />
        超时
      </Badge>
    );
  }
  if (status === "config_error") {
    return (
      <Badge variant="danger" className="gap-1">
        <XCircle className="h-3.5 w-3.5" />
        配置错误
      </Badge>
    );
  }
  if (status === "paused") {
    return (
      <Badge variant="muted" className="gap-1">
        <Clock3 className="h-3.5 w-3.5" />
        已暂停
      </Badge>
    );
  }
  return (
    <Badge variant="muted" className="gap-1">
      <AlertTriangle className="h-3.5 w-3.5" />
      未探测
    </Badge>
  );
}
