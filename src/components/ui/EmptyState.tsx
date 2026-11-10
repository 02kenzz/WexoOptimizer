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
    <div className={cn("empty", className)}>
      <div className="empty-icon">
        <Icon size={20} strokeWidth={1.8} aria-hidden />
      </div>
      <h3>{title}</h3>
      {description ? <p>{description}</p> : null}
      {action ?? null}
    </div>
  );
}
