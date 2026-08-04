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
      aria-sort={active ? (dir === "asc" ? "ascending" : "descending") : "none"}
      style={{ textAlign: "left", padding: 0, color: active ? "var(--color-primary)" : "var(--text-secondary)", fontWeight: 600, userSelect: "none" }}
    >
      <button
        type="button"
        onClick={() => onSort(key)}
        style={{ display: "flex", alignItems: "center", gap: 2, width: "100%", padding: "10px 12px", color: "inherit", font: "inherit", border: 0, background: "transparent", cursor: "pointer", textAlign: "left" }}
      >
        {label}
        <ArrowUpDown size={11} style={{ opacity: active ? 1 : 0.3, transform: active && dir === "desc" ? "scaleY(-1)" : "none" }} />
      </button>
    </th>
  );
}
