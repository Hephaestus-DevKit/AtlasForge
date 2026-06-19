import { useEffect, useRef } from "react";
import { CheckCircle2, XCircle, Info, X } from "lucide-react";

export interface ToastMessage {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

interface ToastContainerProps {
  toasts: ToastMessage[];
  onDismiss: (id: number) => void;
  /** Auto-dismiss timeout in ms. Default 4000. */
  duration?: number;
}

const iconMap = {
  success: CheckCircle2,
  error: XCircle,
  info: Info,
};

const colorMap = {
  success: { bg: "#f0fdf4", border: "#bbf7d0", fg: "#166534", icon: "#10b981" },
  error: { bg: "#fef2f2", border: "#fca5a5", fg: "#991b1b", icon: "#ef4444" },
  info: { bg: "#eff6ff", border: "#bfdbfe", fg: "#1e40af", icon: "#3b82f6" },
};

/**
 * Stacked toast notifications replacing window.alert().
 * Renders in the top-right corner with auto-dismiss.
 */
export function ToastContainer({ toasts, onDismiss, duration = 4000 }: ToastContainerProps) {
  return (
    <div
      style={{
        position: "fixed",
        top: 16,
        right: 16,
        zIndex: 999,
        display: "flex",
        flexDirection: "column",
        gap: 8,
        maxWidth: 380,
      }}
    >
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onDismiss={onDismiss} duration={duration} />
      ))}
    </div>
  );
}

function ToastItem({ toast, onDismiss, duration }: { toast: ToastMessage; onDismiss: (id: number) => void; duration: number }) {
  const Icon = iconMap[toast.type];
  const c = colorMap[toast.type];
  const onDismissRef = useRef(onDismiss);

  useEffect(() => {
    onDismissRef.current = onDismiss;
  }, [onDismiss]);

  useEffect(() => {
    const timer = setTimeout(() => onDismissRef.current(toast.id), duration);
    return () => clearTimeout(timer);
  }, [toast.id, duration]);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "10px 14px",
        background: c.bg,
        border: `1px solid ${c.border}`,
        borderRadius: 6,
        boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
      }}
    >
      <Icon size={16} color={c.icon} />
      <span style={{ flex: 1, fontSize: 13, color: c.fg }}>{toast.message}</span>
      <button
        onClick={() => onDismiss(toast.id)}
        style={{ background: "none", border: "none", cursor: "pointer", padding: 0, color: c.fg, opacity: 0.6 }}
      >
        <X size={14} />
      </button>
    </div>
  );
}
