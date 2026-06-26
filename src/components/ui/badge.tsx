import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const badgeVariants = cva(
  "inline-flex h-6 items-center gap-1.5 rounded-[6px] px-2.5 text-xs font-semibold leading-none [&>svg]:h-3.5 [&>svg]:w-3.5 [&>svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-[#18211d] text-white",
        success: "bg-[#dbf8e7] text-[#176b43] ring-1 ring-[#93d8b1]",
        warning: "bg-[#fff0c2] text-[#85610e] ring-1 ring-[#e7c75e]",
        danger: "bg-[#ffe0e5] text-[#ad2339] ring-1 ring-[#f0a3af]",
        muted: "bg-[#eef4f1] text-[#5b6f64] ring-1 ring-[#d2ded7]",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}
