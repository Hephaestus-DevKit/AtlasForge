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

const typeStyles = {
  success: {
    border: "1px solid var(--color-success-border)",
    iconColor: "var(--color-success-text)",
    glow: "0 4px 20px rgba(16, 185, 129, 0.15)",
  },
  error: {
    border: "1px solid var(--color-danger-border)",
    iconColor: "var(--color-danger-text)",
    glow: "0 4px 20px rgba(239, 68, 68, 0.15)",
  },
  info: {
    border: "1px solid var(--color-info-border)",
    iconColor: "var(--color-info-text)",
    glow: "0 4px 20px rgba(59, 130, 246, 0.15)",
  },
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
        top: 24,
        right: 24,
        zIndex: 9999,
        display: "flex",
        flexDirection: "column",
        gap: 12,
        maxWidth: 380,
        width: "calc(100vw - 48px)",
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
  const style = typeStyles[toast.type];
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
      className="slide-in-right"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "12px 16px",
        background: "var(--bg-card)",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
        border: style.border,
        borderRadius: "var(--radius-md)",
        boxShadow: `var(--shadow-lg), ${style.glow}`,
      }}
    >
      <Icon size={18} color={style.iconColor} style={{ flexShrink: 0 }} />
      <span style={{ flex: 1, fontSize: 13, color: "var(--text-primary)", fontWeight: 500, lineHeight: 1.4 }}>
        {toast.message}
      </span>
      <button
        onClick={() => onDismiss(toast.id)}
        style={{
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: 4,
          color: "var(--text-secondary)",
          opacity: 0.6,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "opacity var(--transition-fast)",
        }}
        onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
        onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.6")}
      >
        <X size={14} />
      </button>
    </div>
  );
}
