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
        padding: "60px 24px",
        gap: 16,
      }}
    >
      <Loader2 size={36} color="var(--color-primary)" style={{ animation: "af-spin 1.2s cubic-bezier(0.5, 0.1, 0.4, 0.9) infinite", filter: "drop-shadow(0 0 8px rgba(99, 102, 241, 0.3))" }} />
      <span style={{ color: "var(--text-secondary)", fontSize: 14, fontWeight: 500 }}>{message}</span>
      <style>{`@keyframes af-spin { to { transform: rotate(360deg) } }`}</style>
    </div>
  );
}
