import { X } from "lucide-react";

interface ConfirmModalProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * In-app confirmation dialog replacing window.confirm().
 * Renders a modal overlay with Cancel / Confirm buttons.
 */
export function ConfirmModal({ message, onConfirm, onCancel }: ConfirmModalProps) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(3, 7, 18, 0.6)",
        backdropFilter: "blur(8px)",
        WebkitBackdropFilter: "blur(8px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 10000,
      }}
      onClick={onCancel}
    >
      <div
        className="card fade-in"
        style={{
          maxWidth: 420,
          width: "90%",
          padding: 24,
          boxShadow: "var(--shadow-lg), 0 0 40px rgba(0, 0, 0, 0.4)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 20 }}>
          <p style={{ fontSize: 15, color: "var(--text-primary)", fontWeight: 500, lineHeight: 1.5, margin: 0 }}>
            {message}
          </p>
          <button
            onClick={onCancel}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              marginLeft: 12,
              color: "var(--text-secondary)",
              opacity: 0.6,
              transition: "opacity var(--transition-fast)",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
            onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.6")}
          >
            <X size={16} />
          </button>
        </div>
        <div style={{ display: "flex", gap: 10, justifyContent: "flex-end" }}>
          <button
            onClick={onCancel}
            className="btn btn-secondary"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="btn btn-danger"
            style={{
              background: "var(--color-danger)",
              color: "#fff",
              border: "none",
              boxShadow: "0 2px 8px rgba(239, 68, 68, 0.3)",
            }}
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}
