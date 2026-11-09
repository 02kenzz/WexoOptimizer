import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon: Icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center rounded-xl border border-dashed border-line/50 px-6 py-12 text-center",
        className,
      )}
    >
      <div className="grid size-12 place-items-center rounded-2xl border border-line bg-card text-mut">
        <Icon className="size-5" strokeWidth={1.8} aria-hidden />
      </div>
      <h3 className="mt-4 text-sm font-medium text-text">{title}</h3>
      <p className="mt-1.5 max-w-md text-xs leading-relaxed text-mut">{description}</p>
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}