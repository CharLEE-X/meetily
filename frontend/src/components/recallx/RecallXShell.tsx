import { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface RecallXShellProps {
  children: ReactNode;
  className?: string;
}

export function RecallXShell({ children, className }: RecallXShellProps) {
  return (
    <div
      className={cn(
        "relative min-h-0 overflow-hidden bg-recallx-black text-recallx-text",
        "before:pointer-events-none before:fixed before:inset-0 before:z-30 before:bg-[repeating-linear-gradient(90deg,rgba(255,255,255,0.55)_0_1px,transparent_1px_4px)] before:opacity-[0.025]",
        "after:pointer-events-none after:absolute after:inset-0 after:bg-[linear-gradient(135deg,rgba(255,255,255,0.045),transparent_34%),repeating-linear-gradient(0deg,transparent_0_27px,rgba(255,255,255,0.035)_28px)]",
        className
      )}
    >
      <div className="relative z-10 flex min-h-0 w-full flex-1">{children}</div>
    </div>
  );
}
