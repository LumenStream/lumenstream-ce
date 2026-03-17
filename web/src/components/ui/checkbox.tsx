import * as React from "react";
import * as CheckboxPrimitives from "@radix-ui/react-checkbox";
import { Check } from "lucide-react";

import { cn } from "@/lib/utils";

const Checkbox = React.forwardRef<
  React.ElementRef<typeof CheckboxPrimitives.Root>,
  React.ComponentPropsWithoutRef<typeof CheckboxPrimitives.Root>
>(({ className, ...props }, ref) => (
  <CheckboxPrimitives.Root
    ref={ref}
    className={cn(
      "peer ring-offset-background focus-visible:ring-ring h-4 w-4 shrink-0 rounded-sm border border-slate-500 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:border-emerald-600 data-[state=checked]:bg-emerald-600 data-[state=checked]:text-white dark:data-[state=checked]:border-emerald-500 dark:data-[state=checked]:bg-emerald-500",
      className
    )}
    {...props}
  >
    <CheckboxPrimitives.Indicator className={cn("flex items-center justify-center text-current")}>
      <Check className="h-3.5 w-3.5" />
    </CheckboxPrimitives.Indicator>
  </CheckboxPrimitives.Root>
));
Checkbox.displayName = CheckboxPrimitives.Root.displayName;

export { Checkbox };
