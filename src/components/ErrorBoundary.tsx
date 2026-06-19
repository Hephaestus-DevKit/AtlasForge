import { Component, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

/**
 * Catches rendering errors in child components and shows a fallback UI
 * instead of a blank white screen. Clicking "Reload" calls location.reload().
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            padding: 48,
            gap: 16,
          }}
        >
          <AlertTriangle size={48} color="#ef4444" />
          <h2 style={{ fontSize: 18, fontWeight: 600, color: "#1e293b" }}>
            Something went wrong
          </h2>
          <p style={{ color: "#64748b", fontSize: 14, maxWidth: 420, textAlign: "center" }}>
            An unexpected error occurred while rendering this page. The error has been logged to the
            developer console.
          </p>
          {this.state.error && (
            <pre
              style={{
                background: "#f8fafc",
                padding: 12,
                borderRadius: 6,
                fontSize: 12,
                color: "#991b1b",
                maxWidth: 600,
                overflow: "auto",
                border: "1px solid #e2e8f0",
              }}
            >
              {this.state.error.message}
            </pre>
          )}
          <button
            onClick={() => window.location.reload()}
            style={{
              padding: "10px 24px",
              background: "#3b82f6",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              cursor: "pointer",
              fontSize: 14,
              fontWeight: 600,
            }}
          >
            Reload Page
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
