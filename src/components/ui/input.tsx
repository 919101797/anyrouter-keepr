import * as React from "react";
import { cn } from "../../lib/utils";

export type InputProps = React.InputHTMLAttributes<HTMLInputElement>;

export const Input = React.forwardRef<HTMLInputElement, InputProps>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn(
      "h-10 w-full rounded-[6px] border border-[#cbd8d1] bg-white px-3 text-sm text-[#17211d] outline-none transition-colors placeholder:text-[#8b9a93] focus:border-[#159a8c] focus:ring-2 focus:ring-[#159a8c]/15 disabled:bg-[#edf3ef] disabled:opacity-70",
      className,
    )}
    {...props}
  />
));
Input.displayName = "Input";
