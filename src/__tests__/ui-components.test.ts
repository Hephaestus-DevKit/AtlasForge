import { describe, expect, it } from "vitest";
import { ErrorBoundary } from "../components/ErrorBoundary";

describe("ErrorBoundary", () => {
  describe("ErrorBoundary", () => {
    it("exports a class component", () => {
      expect(typeof ErrorBoundary).toBe("function");
    });

    it("implements getDerivedStateFromError", () => {
      expect(typeof ErrorBoundary.getDerivedStateFromError).toBe("function");
    });

    it("getDerivedStateFromError returns hasError true with error", () => {
      const error = new Error("test error");
      const state = ErrorBoundary.getDerivedStateFromError(error);
      expect(state).toEqual({ hasError: true, error });
    });

    it("getDerivedStateFromError returns error message correctly", () => {
      const state = ErrorBoundary.getDerivedStateFromError(new Error("boom"));
      expect(state.hasError).toBe(true);
      expect(state.error?.message).toBe("boom");
    });
  });
});
