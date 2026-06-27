import {
  ChartNoAxesColumnIncreasing,
  ChartSpline,
  CircleCheckBig,
  Gauge,
  Hourglass,
  RadioReceiver,
} from "lucide-react";
import type { ComponentType } from "react";
import { formatDuration, formatLongDuration, formatRelativeTime } from "../lib/utils";
import type { AppStatus, ProbeEvent } from "../lib/types";

interface StatStripProps {
  status: AppStatus | null;
  events: ProbeEvent[];
}

const iconTone = [
  "bg-[#e6f8ee] text-[#177448]",
  "bg-[#eaf3ff] text-[#2467a8]",
  "bg-[#fff0c2] text-[#8b650c]",
  "bg-[#e9f4ef] text-[#52645b]",
  "bg-[#f0edff] text-[#5c4e9e]",
  "bg-[#ffe7df] text-[#b84d2f]",
];

export function StatStrip({ status, events }: StatStripProps) {
  const success = events.filter((event) => event.status === "success").length;
  const queueMiss = events.filter((event) => event.status === "queue_miss").length;
  const successRate = events.length ? Math.round((success / events.length) * 100) : 0;
  const queueMissRate = events.length ? Math.round((queueMiss / events.length) * 100) : 0;
  const longestNoSuccessMs = longestNoSuccessWindow(events);

  const stats = [
    {
      label: "最近成功",
      value: formatRelativeTime(status?.last_success_at),
      icon: CircleCheckBig,
    },
    {
      label: "最近耗时",
      value: formatDuration(status?.last_event?.duration_ms),
      icon: Gauge,
    },
    {
      label: "连续抢占",
      value: String(status?.consecutive_queue_miss ?? 0),
      icon: RadioReceiver,
    },
    {
      label: "近端成功率",
      value: `${successRate}%`,
      icon: ChartNoAxesColumnIncreasing,
    },
    {
      label: "抢占占比",
      value: `${queueMissRate}%`,
      icon: ChartSpline,
    },
    {
      label: "最长未成功",
      value: formatLongDuration(longestNoSuccessMs),
      icon: Hourglass,
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-3 2xl:grid-cols-6">
      {stats.map((stat, index) => (
        <StatCard
          key={stat.label}
          icon={stat.icon}
          tone={iconTone[index % iconTone.length]}
          label={stat.label}
          value={stat.value}
        />
      ))}
    </div>
  );
}

function StatCard({
  icon: Icon,
  tone,
  label,
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  tone: string;
  label: string;
  value: string;
}) {
  return (
    <div className="panel-ring min-h-[112px] rounded-[8px] border border-[#d2ded7] bg-white p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-xs font-semibold uppercase tracking-[0.08em] text-[#6a7d73]">{label}</div>
          <div className="mt-3 truncate text-2xl font-black tracking-normal text-[#17211d]">{value}</div>
        </div>
        <div className={`stat-card-icon ${tone}`}>
          <Icon className="h-4 w-4" />
        </div>
      </div>
    </div>
  );
}

function longestNoSuccessWindow(events: ProbeEvent[]) {
  if (!events.length) return null;

  const ordered = events
    .map((event) => ({ ...event, time: new Date(event.started_at).getTime() }))
    .filter((event) => Number.isFinite(event.time))
    .sort((left, right) => left.time - right.time);

  if (!ordered.length) return null;

  let lastSuccessAt = ordered[0].time;
  let longest = 0;

  for (const event of ordered) {
    if (event.status === "success") {
      longest = Math.max(longest, event.time - lastSuccessAt);
      lastSuccessAt = event.time;
    }
  }

  longest = Math.max(longest, Date.now() - lastSuccessAt);
  return longest;
}
