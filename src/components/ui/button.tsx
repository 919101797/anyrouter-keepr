import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const buttonVariants = cva(
  "inline-flex h-10 cursor-pointer items-center justify-center gap-2 whitespace-nowrap rounded-[6px] px-4 text-sm font-semibold leading-none transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#111815]/20 disabled:pointer-events-none disabled:opacity-50 [&>svg]:h-4 [&>svg]:w-4 [&>svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-[#121815] text-white shadow-[0_10px_24px_rgba(17,24,21,0.18)] hover:bg-[#24322c]",
        secondary:
          "border border-[#cbd8d1] bg-white text-[#18211d] hover:border-[#9db2a8] hover:bg-[#f6faf7]",
        ghost: "text-[#52645b] hover:bg-[#e5eee9] hover:text-[#17211d]",
        destructive: "bg-[#e5485d] text-white hover:bg-[#d3394f]",
        outline: "border border-[#cbd8d1] bg-[#f8fbf9] text-[#1e2b25] hover:border-[#8fa79b] hover:bg-white",
      },
      size: {
        default: "h-10 px-4",
        sm: "h-8 px-3 text-xs",
        icon: "h-10 w-10 p-0",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props} />;
  },
);
Button.displayName = "Button";
