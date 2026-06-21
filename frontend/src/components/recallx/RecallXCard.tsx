import { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface RecallXCardProps {
  children: ReactNode;
  className?: string;
  innerClassName?: string;
}

export function RecallXCard({ children, className, innerClassName }: RecallXCardProps) {
  return (
    <div className={cn("rounded-lg bg-white/[0.06] p-1.5 ring-1 ring-white/10", className)}>
      <div
        className={cn(
          "h-full rounded-md bg-recallx-graphite shadow-[inset_0_1px_1px_rgba(255,255,255,0.14)]",
          innerClassName
        )}
      >
        {children}
      </div>
    </div>
  );
}
