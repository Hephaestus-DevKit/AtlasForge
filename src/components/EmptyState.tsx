import type { LucideIcon } from "lucide-react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: React.ReactNode;
}

/**
 * Consistent empty state replacing ad-hoc "No X yet" blocks across pages.
 * Renders icon + title + optional description + optional action button/element.
 */
export function EmptyState({ icon: Icon, title, description, action }: EmptyStateProps) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: 48,
        gap: 8,
      }}
    >
      <Icon size={40} color="#94a3b8" strokeWidth={1.5} style={{ marginBottom: 4, opacity: 0.6 }} />
      <h2 style={{ fontSize: 16, fontWeight: 600, color: "#334155", margin: 0 }}>{title}</h2>
      {description && (
        <p style={{ color: "#64748b", fontSize: 13, textAlign: "center", maxWidth: 400, margin: 0 }}>
          {description}
        </p>
      )}
      {action && <div style={{ marginTop: 8 }}>{action}</div>}
    </div>
  );
}
