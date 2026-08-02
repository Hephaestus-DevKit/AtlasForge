import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export function ProfileSection({ icon: Icon, title, color, children }: { icon: LucideIcon; title: string; color: string; children: ReactNode }) {
  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <Icon size={14} color={color} />
        <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{title}</span>
      </div>
      <div style={{ paddingLeft: 20 }}>{children}</div>
    </div>
  );
}

export function Tag({ label, color }: { label: string; color: string }) {
  return (
    <span style={{ padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, background: color + "18", color, border: `1px solid ${color}33` }}>
      {label}
    </span>
  );
}

export function MiniBadge({ label, active }: { label: string; active: boolean }) {
  return <span className={`badge ${active ? "badge-success" : "badge-neutral"}`}>{active ? `✓ ${label}` : `✗ ${label}`}</span>;
}

export function SeverityBadge({ severity }: { severity: string }) {
  let badgeClass = "badge-neutral";
  if (severity === "critical" || severity === "high") badgeClass = "badge-danger";
  else if (severity === "medium") badgeClass = "badge-warning";
  else if (severity === "low") badgeClass = "badge-info";
  return <span className={`badge ${badgeClass}`} style={{ textTransform: "lowercase" }}>{severity}</span>;
}

export function PatchStatusBadge({ status }: { status: string }) {
  let badgeClass = "badge-neutral";
  if (status === "proposed") badgeClass = "badge-info";
  else if (status === "applied") badgeClass = "badge-success";
  else if (status === "rejected") badgeClass = "badge-danger";
  else if (status === "rolled_back") badgeClass = "badge-warning";
  return <span className={`badge ${badgeClass}`} style={{ marginLeft: 8, textTransform: "lowercase" }}>{status}</span>;
}

export function EmptyText({ children }: { children: ReactNode }) {
  return <p style={{ fontSize: 11, color: "var(--text-muted)", fontStyle: "italic" }}>{children}</p>;
}
