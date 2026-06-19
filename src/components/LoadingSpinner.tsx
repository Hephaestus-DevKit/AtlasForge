import { Loader2 } from "lucide-react";

/**
 * Consistent loading indicator replacing inline "Loading..." text.
 * Use on every page that fetches data on mount.
 */
export function LoadingSpinner({ message = "Loading..." }: { message?: string }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: 48,
        gap: 12,
      }}
    >
      <Loader2 size={32} color="#94a3b8" style={{ animation: "af-spin 1s linear infinite" }} />
      <span style={{ color: "#94a3b8", fontSize: 14 }}>{message}</span>
      <style>{`@keyframes af-spin { to { transform: rotate(360deg) } }`}</style>
    </div>
  );
}
