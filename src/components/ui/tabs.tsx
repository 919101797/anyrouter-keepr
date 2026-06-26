import * as TabsPrimitive from "@radix-ui/react-tabs";
import { cn } from "../../lib/utils";

export const Tabs = TabsPrimitive.Root;

export function TabsList({ className, ...props }: TabsPrimitive.TabsListProps) {
  return (
    <TabsPrimitive.List
      className={cn("inline-flex rounded-[8px] border border-[#cbd8d1] bg-[#e9f1ed] p-1", className)}
      {...props}
    />
  );
}

export function TabsTrigger({ className, ...props }: TabsPrimitive.TabsTriggerProps) {
  return (
    <TabsPrimitive.Trigger
      className={cn(
        "h-9 cursor-pointer rounded-[6px] px-3 text-sm font-semibold text-[#617369] transition-colors hover:bg-[#f4f9f6] hover:text-[#17211d] data-[state=active]:bg-white data-[state=active]:text-[#121815] data-[state=active]:shadow-sm",
        className,
      )}
      {...props}
    />
  );
}

export function TabsContent({ className, ...props }: TabsPrimitive.TabsContentProps) {
  return <TabsPrimitive.Content className={cn("mt-5", className)} {...props} />;
}
