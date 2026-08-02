import { ArrowUpDown } from "lucide-react";

export type SortKey = "path" | "branch" | "dirty" | "lastCommit" | "score" | "language";
export type SortDir = "asc" | "desc";

export function SortableHeader({ label, sortKey: key, currentKey, dir, onSort }: {
  label: string;
  sortKey: SortKey;
  currentKey: SortKey;
  dir: SortDir;
  onSort: (key: SortKey) => void;
}) {
  const active = currentKey === key;
  return (
    <th
      style={{ textAlign: "left", padding: "10px 12px", color: active ? "var(--color-primary)" : "var(--text-secondary)", fontWeight: 600, cursor: "pointer", userSelect: "none" }}
      onClick={() => onSort(key)}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
        {label}
        <ArrowUpDown size={11} style={{ opacity: active ? 1 : 0.3, transform: active && dir === "desc" ? "scaleY(-1)" : "none" }} />
      </div>
    </th>
  );
}
