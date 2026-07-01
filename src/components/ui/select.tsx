import * as SelectPrimitive from "@radix-ui/react-select";
import type { ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

export const Select = SelectPrimitive.Root;
export const SelectValue = SelectPrimitive.Value;
export const SELECT_CONTENT_POSITION = "popper" satisfies SelectPrimitive.SelectContentProps["position"];
export const SELECT_CONTENT_POSITIONING_CLASS =
  "fixed max-h-[min(24rem,var(--radix-select-content-available-height))] min-w-[var(--radix-select-trigger-width)]";
const SELECT_CONTENT_SIDE_OFFSET = 6;

export function SelectTrigger({
  className,
  children,
  ...props
}: SelectPrimitive.SelectTriggerProps & { children: ReactNode }) {
  return (
    <SelectPrimitive.Trigger
      className={cn(
        "select-trigger flex h-10 w-full min-w-0 cursor-pointer items-center justify-between rounded-[6px] border border-[#cbd8d1] bg-white px-3 text-sm text-[#17211d] outline-none transition-colors focus:border-[#159a8c] focus:ring-2 focus:ring-[#159a8c]/15",
        className,
      )}
      {...props}
    >
      {children}
      <SelectPrimitive.Icon asChild>
        <ChevronDown className="h-4 w-4 shrink-0 text-[#65766e]" />
      </SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
  );
}

export function SelectContent({
  className,
  children,
  position = SELECT_CONTENT_POSITION,
  sideOffset = SELECT_CONTENT_SIDE_OFFSET,
  ...props
}: SelectPrimitive.SelectContentProps) {
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        position={position}
        sideOffset={sideOffset}
        className={cn(
          "select-content z-50 overflow-hidden rounded-[6px] border border-[#cbd8d1] bg-white text-[#17211d] shadow-xl",
          SELECT_CONTENT_POSITIONING_CLASS,
          className,
        )}
        {...props}
      >
        <SelectPrimitive.Viewport className="p-1">{children}</SelectPrimitive.Viewport>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  );
}

export function SelectItem({ className, children, ...props }: SelectPrimitive.SelectItemProps) {
  return (
    <SelectPrimitive.Item
      className={cn(
        "select-item relative flex cursor-pointer select-none items-center rounded-[4px] px-8 py-2 text-sm outline-none",
        className,
      )}
      {...props}
    >
      <span className="absolute left-2 flex h-4 w-4 items-center justify-center">
        <SelectPrimitive.ItemIndicator>
          <Check className="h-4 w-4 text-[#159a8c]" />
        </SelectPrimitive.ItemIndicator>
      </span>
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
    </SelectPrimitive.Item>
  );
}
