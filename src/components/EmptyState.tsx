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
        padding: "60px 24px",
        gap: 12,
        background: "rgba(255, 255, 255, 0.01)",
        border: "1px dashed var(--border-color)",
        borderRadius: "var(--radius-lg)",
        margin: "20px 0",
      }}
    >
      <Icon size={44} color="var(--color-primary)" strokeWidth={1.5} style={{ marginBottom: 4, opacity: 0.8, filter: "drop-shadow(0 0 8px rgba(99, 102, 241, 0.3))" }} />
      <h2 style={{ fontSize: 16, fontWeight: 700, color: "var(--text-primary)", margin: 0 }}>{title}</h2>
      {description && (
        <p style={{ color: "var(--text-secondary)", fontSize: 13, textAlign: "center", maxWidth: 420, margin: 0, lineHeight: 1.5 }}>
          {description}
        </p>
      )}
      {action && <div style={{ marginTop: 12 }}>{action}</div>}
    </div>
  );
}
