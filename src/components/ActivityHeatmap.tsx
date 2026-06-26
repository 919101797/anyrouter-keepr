import { Activity } from "lucide-react";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";
import type { ActivityBucket, ProbeStatus } from "../lib/types";
import { cn, formatClock, formatDuration } from "../lib/utils";

interface ActivityHeatmapProps {
  buckets: ActivityBucket[];
  anchorTime: number;
}

function bucketColor(status?: ProbeStatus | null) {
  switch (status) {
    case "success":
      return "bg-[#2cc875] shadow-[0_0_16px_rgba(44,200,117,0.28)]";
    case "queue_miss":
      return "bg-[#f1bd35]";
    case "timeout":
      return "bg-[#f07738]";
    case "config_error":
      return "bg-[#e5485d]";
    case "unknown":
      return "bg-[#93a59c]";
    default:
      return "bg-[#dfe8e3]";
  }
}

function strongestStatus(bucket: ActivityBucket): ProbeStatus | null {
  if (bucket.config_error_count) return "config_error";
  if (bucket.success_count) return "success";
  if (bucket.timeout_count) return "timeout";
  if (bucket.queue_miss_count) return "queue_miss";
  if (bucket.unknown_count) return "unknown";
  return null;
}

export function ActivityHeatmap({ buckets, anchorTime }: ActivityHeatmapProps) {
  const bucketMap = new Map(buckets.map((bucket) => [new Date(bucket.bucket_start).getTime(), bucket]));
  const slots = Array.from({ length: 288 }, (_, index) => {
    const time = new Date(anchorTime - (287 - index) * 5 * 60_000);
    time.setSeconds(0, 0);
    time.setMinutes(time.getMinutes() - (time.getMinutes() % 5));
    const bucket = bucketMap.get(time.getTime());
    return { time, bucket };
  });

  return (
    <TooltipProvider>
      <div className="panel-ring rounded-[8px] border border-[#d2ded7] bg-white p-5">
        <div className="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[7px] bg-[#e6f8ee] text-[#177448]">
              <Activity className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <h2 className="text-sm font-bold text-[#121815]">24 小时活性</h2>
              <p className="mt-0.5 text-xs font-medium text-[#66796f]">每格 5 分钟</p>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-2 text-xs font-semibold text-[#617369]">
            <Legend color="bg-[#2cc875]" label="成功" />
            <Legend color="bg-[#f1bd35]" label="抢占" />
            <Legend color="bg-[#f07738]" label="超时" />
            <Legend color="bg-[#e5485d]" label="配置" />
          </div>
        </div>
        <div className="overflow-x-auto pb-1">
          <div
            className="grid min-w-[720px] gap-1"
            style={{ gridTemplateColumns: "repeat(72, minmax(0, 1fr))" }}
          >
            {slots.map(({ time, bucket }, index) => {
              const status = bucket ? strongestStatus(bucket) : null;
              return (
                <Tooltip key={`${time.toISOString()}-${index}`}>
                  <TooltipTrigger asChild>
                    <div
                      className={cn(
                        "h-3 rounded-[3px] transition-transform hover:scale-y-150 hover:rounded-[4px]",
                        bucketColor(status),
                      )}
                    />
                  </TooltipTrigger>
                  <TooltipContent>
                    <div className="space-y-1">
                      <div className="font-semibold">{formatClock(time.toISOString())}</div>
                      {bucket ? (
                        <>
                          <div>
                            成功 {bucket.success_count} / 抢占 {bucket.queue_miss_count}
                          </div>
                          <div>
                            超时 {bucket.timeout_count} / 配置 {bucket.config_error_count}
                          </div>
                          <div>最近耗时 {formatDuration(bucket.last_latency_ms)}</div>
                        </>
                      ) : (
                        <div>无 probe 数据</div>
                      )}
                    </div>
                  </TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </div>
      </div>
    </TooltipProvider>
  );
}

function Legend({ color, label }: { color: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={cn("h-2.5 w-2.5 rounded-[3px]", color)} />
      {label}
    </span>
  );
}
