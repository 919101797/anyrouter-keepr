import { Fragment, useCallback, useMemo, useState } from "react";
import { createColumnHelper, flexRender, getCoreRowModel, useReactTable } from "@tanstack/react-table";
import {
  BadgeAlert,
  ChevronDown,
  CircleAlert,
  CircleCheckBig,
  Clock,
  FileClock,
  ListFilter,
  RadioReceiver,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { PaginationBar } from "./PaginationBar";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { StatusPill } from "./StatusPill";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import type { ProbeEvent } from "../lib/types";
import { cn, formatClock, formatDuration, statusLabel } from "../lib/utils";

interface ProbeHistoryTableProps {
  events: ProbeEvent[];
  filter: string;
  onFilter: (filter: string) => void;
}

const PAGE_SIZE = 10;
const STATUS_FILTERS = new Set(["success", "queue_miss", "timeout", "config_error", "unknown"]);

const helper = createColumnHelper<ProbeEvent>();

export function ProbeHistoryTable({ events, filter, onFilter }: ProbeHistoryTableProps) {
  const [page, setPage] = useState(0);
  const [expandedEventId, setExpandedEventId] = useState<string | null>(null);
  const filteredEvents = useMemo(() => filterEvents(events, filter), [events, filter]);
  const pageCount = Math.max(1, Math.ceil(filteredEvents.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const pagedEvents = useMemo(
    () => filteredEvents.slice(safePage * PAGE_SIZE, (safePage + 1) * PAGE_SIZE),
    [filteredEvents, safePage],
  );
  const toggleExpanded = useCallback((eventId: string) => {
    setExpandedEventId((current) => (current === eventId ? null : eventId));
  }, []);
  const columns = useMemo(
    () => createColumns(expandedEventId, toggleExpanded),
    [expandedEventId, toggleExpanded],
  );
  const table = useReactTable({
    data: pagedEvents,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const filters: Array<{ value: string; label: string; icon: LucideIcon }> = [
    { value: "all", label: "全部", icon: ListFilter },
    { value: "success", label: "成功", icon: CircleCheckBig },
    { value: "queue_miss", label: "抢占", icon: RadioReceiver },
    { value: "timeout", label: "超时", icon: Clock },
    { value: "config_error", label: "配置", icon: BadgeAlert },
    { value: "unknown", label: "未判定", icon: CircleAlert },
    { value: "recent_1h", label: "近 1 小时", icon: FileClock },
    { value: "recent_24h", label: "近 24 小时", icon: FileClock },
  ];

  const handlePageChange = (nextPage: number) => {
    setExpandedEventId(null);
    setPage(nextPage);
  };

  return (
    <div className="panel-ring overflow-hidden rounded-[8px] border border-[#d2ded7] bg-white">
      <div className="flex flex-col gap-3 border-b border-[#dce7e1] px-5 py-4 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <div className="min-w-0">
            <h2 className="text-sm font-bold text-[#121815]">守护历史</h2>
            <p className="mt-0.5 text-xs font-medium text-[#66796f]">
              每次调用的状态、输入输出与 stdout / stderr 详情
            </p>
          </div>
          <Badge variant="muted">{filteredEvents.length ? `${filteredEvents.length} 条` : "等待事件"}</Badge>
        </div>
        <div className="flex flex-wrap gap-1.5">
          {filters.map((filterItem) => {
            const Icon = filterItem.icon;
            return (
              <Button
                key={filterItem.value}
                size="sm"
                variant={filter === filterItem.value ? "default" : "ghost"}
                onClick={() => {
                  setPage(0);
                  setExpandedEventId(null);
                  onFilter(filterItem.value);
                }}
                className={filter === filterItem.value ? "history-filter-active" : "history-filter-button"}
              >
                <Icon />
                {filterItem.label}
              </Button>
            );
          })}
        </div>
      </div>
      <div className="max-h-[520px] overflow-auto">
        <Table className="min-w-[1280px]">
          <TableHeader className="history-table-header sticky top-0 z-10 bg-white">
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id} className="history-table-head-row">
                {headerGroup.headers.map((header) => (
                  <TableHead key={header.id}>
                    {header.isPlaceholder
                      ? null
                      : flexRender(header.column.columnDef.header, header.getContext())}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows.length ? (
              table.getRowModel().rows.map((row) => {
                const event = row.original;
                const expanded = expandedEventId === event.id;
                return (
                  <Fragment key={row.id}>
                    <TableRow className={cn("history-table-row", expanded && "history-table-row-expanded")}>
                      {row.getVisibleCells().map((cell) => (
                        <TableCell key={cell.id}>
                          {flexRender(cell.column.columnDef.cell, cell.getContext())}
                        </TableCell>
                      ))}
                    </TableRow>
                    {expanded ? (
                      <TableRow className="history-detail-row">
                        <TableCell colSpan={columns.length} className="history-detail-cell p-3">
                          <EventDetail event={event} />
                        </TableCell>
                      </TableRow>
                    ) : null}
                  </Fragment>
                );
              })
            ) : (
              <TableRow>
                <TableCell colSpan={columns.length} className="h-24 text-center text-[#66796f]">
                  暂无守护记录
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
      <PaginationBar
        page={safePage}
        pageSize={PAGE_SIZE}
        total={filteredEvents.length}
        onPageChange={handlePageChange}
      />
    </div>
  );
}

function createColumns(expandedEventId: string | null, toggleExpanded: (eventId: string) => void) {
  return [
    helper.accessor("started_at", {
      header: "开始",
      cell: (info) => <span className="mono text-xs font-semibold">{formatClock(info.getValue())}</span>,
    }),
    helper.accessor("ended_at", {
      header: "结束",
      cell: (info) => <span className="mono text-xs font-semibold">{formatClock(info.getValue())}</span>,
    }),
    helper.accessor("status", {
      header: "状态",
      cell: (info) => (
        <div className="history-status-cell">
          <StatusPill status={info.getValue()} />
        </div>
      ),
    }),
    helper.accessor("error_kind", {
      header: "错误类型",
      cell: (info) => (
        <span className="mono text-xs font-semibold text-[#617369]">{info.getValue() ?? "-"}</span>
      ),
    }),
    helper.accessor("duration_ms", {
      header: "耗时",
      cell: (info) => <span className="mono text-xs font-semibold">{formatDuration(info.getValue())}</span>,
    }),
    helper.accessor("exit_code", {
      header: "Exit",
      cell: (info) => (
        <span className="mono text-xs font-semibold text-[#617369]">
          {info.getValue() == null ? "-" : info.getValue()}
        </span>
      ),
    }),
    helper.accessor("model", {
      header: "模型",
      cell: (info) => (
        <span className="mono inline-block max-w-[150px] truncate text-xs font-semibold">
          {info.getValue()?.trim() || "默认模型"}
        </span>
      ),
    }),
    helper.accessor("base_url", {
      header: "Endpoint",
      cell: (info) => (
        <span className="line-clamp-1 max-w-[190px] text-xs font-medium text-[#617369]">
          {info.getValue()?.trim() || "Claude Code 当前配置"}
        </span>
      ),
    }),
    helper.display({
      id: "prompt_summary",
      header: "输入",
      cell: (info) => (
        <HistoryTextCell
          value={info.row.original.prompt_summary}
          truncated={info.row.original.prompt_truncated}
          className="max-w-[220px]"
        />
      ),
    }),
    helper.display({
      id: "output_summary",
      header: "输出",
      cell: (info) => (
        <HistoryTextCell
          value={outputSummary(info.row.original)}
          truncated={info.row.original.stdout_truncated || info.row.original.stderr_truncated}
          className="max-w-[260px]"
        />
      ),
    }),
    helper.display({
      id: "detail",
      header: "详情",
      cell: (info) => {
        const event = info.row.original;
        const expanded = expandedEventId === event.id;
        return (
          <Button
            type="button"
            size="sm"
            variant={expanded ? "secondary" : "outline"}
            className="px-2.5"
            onClick={() => toggleExpanded(event.id)}
          >
            <ChevronDown className={cn("transition-transform", expanded && "rotate-180")} />
            详情
          </Button>
        );
      },
    }),
  ];
}

function filterEvents(events: ProbeEvent[], filter: string) {
  if (STATUS_FILTERS.has(filter)) return events.filter((event) => event.status === filter);
  if (filter !== "recent_1h" && filter !== "recent_24h") return events;
  const windowMs = filter === "recent_1h" ? 60 * 60_000 : 24 * 60 * 60_000;
  const since = Date.now() - windowMs;
  return events.filter((event) => new Date(event.started_at).getTime() >= since);
}

function EventDetail({ event }: { event: ProbeEvent }) {
  const exitCode = event.exit_code == null ? "无" : String(event.exit_code);
  const errorKind = event.error_kind?.trim() || "无";
  const model = event.model?.trim() || "默认模型";
  const endpoint = event.base_url?.trim() || "Claude Code 当前配置";

  return (
    <div className="rounded-[7px] border border-[#16211c] bg-[#101612] p-3 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]">
      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-6">
        <DetailFact label="状态" value={statusLabel(event.status)} />
        <DetailFact label="开始" value={formatClock(event.started_at)} />
        <DetailFact label="结束" value={formatClock(event.ended_at)} />
        <DetailFact label="耗时" value={formatDuration(event.duration_ms)} />
        <DetailFact label="Exit" value={exitCode} />
        <DetailFact label="错误" value={errorKind} />
      </div>
      <div className="mt-2 grid gap-2 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <DetailFact label="模型" value={model} />
        <DetailFact label="Endpoint" value={endpoint} />
      </div>
      <div className="mt-2 grid gap-2 xl:grid-cols-3">
        <DetailTextBlock label="输入" value={event.prompt_summary} truncated={event.prompt_truncated} />
        <DetailTextBlock label="stdout" value={event.stdout_summary} truncated={event.stdout_truncated} />
        <DetailTextBlock label="stderr" value={event.stderr_summary} truncated={event.stderr_truncated} />
      </div>
    </div>
  );
}

function DetailFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-[6px] border border-white/10 bg-white/[0.035] px-2.5 py-2">
      <div className="text-[10px] font-bold uppercase tracking-[0.12em] text-[#7f948a]">{label}</div>
      <div className="mt-1 truncate text-xs font-semibold text-[#dce8e2]">{value}</div>
    </div>
  );
}

function DetailTextBlock({
  label,
  value,
  truncated,
}: {
  label: string;
  value?: string | null;
  truncated: boolean;
}) {
  return (
    <div className="mono min-w-0 rounded-[6px] border border-white/10 bg-black/20 px-3 py-2 text-xs font-semibold leading-5 text-[#dce8e2]">
      <div className="mb-1 flex items-center justify-between gap-2">
        <span className="text-[#8fa39a]">{label}</span>
        <span className={truncated ? "text-[#ffd45d]" : "text-[#617369]"}>
          {truncated ? "已截断" : "完整"}
        </span>
      </div>
      <div className="max-h-28 overflow-auto whitespace-pre-wrap break-words">{value?.trim() || "-"}</div>
    </div>
  );
}

function outputSummary(event: ProbeEvent) {
  const stdout = event.stdout_summary?.trim();
  const stderr = event.stderr_summary?.trim();
  if (stdout && stderr) return `stdout: ${stdout}\nstderr: ${stderr}`;
  if (stdout) return stdout;
  if (stderr) return stderr;
  return "";
}

function HistoryTextCell({
  value,
  truncated,
  className,
}: {
  value?: string | null;
  truncated: boolean;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const text = value?.trim();
  if (!text) return <span className="text-xs font-medium text-[#9aaca3]">-</span>;

  const tooltipText = truncated ? `${text}\n\n（后端摘要已截断）` : text;

  return (
    <Tooltip open={open} onOpenChange={setOpen}>
      <TooltipTrigger asChild>
        <button
          type="button"
          onBlur={() => window.setTimeout(() => setOpen(false), 120)}
          onClick={() => setOpen((current) => !current)}
          onKeyDown={(event) => {
            if (event.key === "Escape") setOpen(false);
          }}
          className={cn(
            "history-text-cell block truncate text-left text-xs font-medium underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#111815]/20",
            className,
          )}
        >
          {text}
        </button>
      </TooltipTrigger>
      <TooltipContent className="max-w-[520px] whitespace-pre-wrap break-words px-3 py-2 leading-5">
        {tooltipText}
      </TooltipContent>
    </Tooltip>
  );
}
