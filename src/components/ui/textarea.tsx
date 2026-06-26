import * as React from "react";
import { cn } from "../../lib/utils";

export type TextareaProps = React.TextareaHTMLAttributes<HTMLTextAreaElement>;

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, ...props }, ref) => (
    <textarea
      ref={ref}
      className={cn(
        "min-h-24 w-full resize-y rounded-[6px] border border-[#cbd8d1] bg-white px-3 py-2 text-sm text-[#17211d] outline-none transition-colors placeholder:text-[#8b9a93] focus:border-[#159a8c] focus:ring-2 focus:ring-[#159a8c]/15 disabled:bg-[#edf3ef] disabled:opacity-70",
        className,
      )}
      {...props}
    />
  ),
);
Textarea.displayName = "Textarea";
