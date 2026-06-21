import { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface RecallXButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  trailing?: ReactNode;
}

export function RecallXButton({ children, trailing = "↗", className, ...props }: RecallXButtonProps) {
  return (
    <button
      className={cn(
        "group inline-flex items-center gap-3 rounded-full bg-recallx-text py-1.5 pl-5 pr-1.5 text-sm font-semibold text-recallx-black transition-all duration-700 active:scale-[0.98]",
        "recallx-ease disabled:pointer-events-none disabled:opacity-50",
        className
      )}
      {...props}
    >
      <span>{children}</span>
      <span className="flex h-8 w-8 items-center justify-center rounded-full bg-recallx-acid text-recallx-black transition-transform duration-700 recallx-ease group-hover:translate-x-1 group-hover:-translate-y-px group-hover:scale-105">
        {trailing}
      </span>
    </button>
  );
}
