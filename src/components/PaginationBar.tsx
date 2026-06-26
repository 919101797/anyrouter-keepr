import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "./ui/button";
import { cn } from "../lib/utils";

interface PaginationBarProps {
  page: number;
  pageSize: number;
  total: number;
  tone?: "light" | "dark";
  onPageChange: (page: number) => void;
}

export function PaginationBar({ page, pageSize, total, tone = "light", onPageChange }: PaginationBarProps) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const safePage = Math.min(page, pageCount - 1);
  const start = total === 0 ? 0 : safePage * pageSize + 1;
  const end = Math.min(total, (safePage + 1) * pageSize);
  const dark = tone === "dark";

  return (
    <div
      className={cn(
        "flex flex-col gap-2 border-t px-3 py-2.5 text-xs font-semibold sm:flex-row sm:items-center sm:justify-between",
        dark
          ? "border-white/10 bg-white/[0.035] text-[#91a49b]"
          : "border-[#dce7e1] bg-[#f8fbf9] text-[#66796f]",
      )}
    >
      <span>
        {start}-{end} / {total} · 每页 {pageSize} 条
      </span>
      <div className="flex items-center gap-2">
        <Button
          type="button"
          size="icon"
          variant={dark ? "ghost" : "outline"}
          className={cn("h-8 w-8", dark && "border-white/10 bg-white/5 text-white hover:bg-white/10")}
          disabled={safePage <= 0}
          onClick={() => onPageChange(Math.max(0, safePage - 1))}
        >
          <ChevronLeft />
        </Button>
        <span className={cn("mono min-w-[62px] text-center", dark ? "text-[#dce8e2]" : "text-[#17211d]")}>
          {safePage + 1} / {pageCount}
        </span>
        <Button
          type="button"
          size="icon"
          variant={dark ? "ghost" : "outline"}
          className={cn("h-8 w-8", dark && "border-white/10 bg-white/5 text-white hover:bg-white/10")}
          disabled={safePage >= pageCount - 1}
          onClick={() => onPageChange(Math.min(pageCount - 1, safePage + 1))}
        >
          <ChevronRight />
        </Button>
      </div>
    </div>
  );
}
