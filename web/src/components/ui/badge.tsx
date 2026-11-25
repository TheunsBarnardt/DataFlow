import type * as React from "react";
import { cn } from "@/lib/utils";

const variants: Record<string, string> = {
  default: "",
  secondary: "bg-muted text-muted-foreground",
  // !border-* needed: unlayered `* { border-color }` in index.css beats Tailwind utilities
  success:
    "!border-emerald-500 text-emerald-700 dark:text-emerald-400",
  destructive: "!border-destructive text-destructive",
};

export function Badge({
  className,
  variant = "default",
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & {
  variant?: keyof typeof variants;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium",
        variants[variant] ?? "",
        className,
      )}
      {...props}
    />
  );
}
