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
        background: "rgba(0,0,0,0.3)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
      onClick={onCancel}
    >
      <div
        style={{
          background: "#fff",
          borderRadius: 8,
          padding: 24,
          maxWidth: 420,
          width: "90%",
          boxShadow: "0 8px 30px rgba(0,0,0,0.12)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 12 }}>
          <p style={{ fontSize: 14, color: "#1e293b", lineHeight: 1.5, margin: 0 }}>{message}</p>
          <button
            onClick={onCancel}
            style={{ background: "none", border: "none", cursor: "pointer", padding: 0, marginLeft: 12, color: "#94a3b8" }}
          >
            <X size={16} />
          </button>
        </div>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button
            onClick={onCancel}
            style={{
              padding: "8px 16px",
              background: "#f1f5f9",
              color: "#475569",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 13,
            }}
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            style={{
              padding: "8px 16px",
              background: "#ef4444",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 13,
              fontWeight: 600,
            }}
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}
