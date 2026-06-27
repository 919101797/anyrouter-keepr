import {
  CircleAlert,
  CircleCheckBig,
  CirclePause,
  CircleX,
  Clock,
  Radar,
  RadioReceiver,
} from "lucide-react";
import { Badge } from "./ui/badge";
import type { ProbeStatus } from "../lib/types";

export function StatusPill({ status }: { status?: ProbeStatus | string | null }) {
  if (status === "success" || status === "connected") {
    return (
      <Badge variant="success" className="status-pill">
        <CircleCheckBig />
        已联通
      </Badge>
    );
  }
  if (status === "queue_miss" || status === "racing") {
    return (
      <Badge variant="warning" className="status-pill">
        <RadioReceiver />
        抢占中
      </Badge>
    );
  }
  if (status === "probing") {
    return (
      <Badge variant="warning" className="status-pill">
        <Radar />
        探测中
      </Badge>
    );
  }
  if (status === "timeout") {
    return (
      <Badge variant="warning" className="status-pill">
        <Clock />
        超时
      </Badge>
    );
  }
  if (status === "config_error") {
    return (
      <Badge variant="danger" className="status-pill">
        <CircleX />
        配置错误
      </Badge>
    );
  }
  if (status === "paused") {
    return (
      <Badge variant="muted" className="status-pill">
        <CirclePause />
        已暂停
      </Badge>
    );
  }
  return (
    <Badge variant="muted" className="status-pill">
      <CircleAlert />
      未判定
    </Badge>
  );
}
