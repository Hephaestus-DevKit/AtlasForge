import { describe, expect, it } from "vitest";
import { LoadingSpinner } from "../components/LoadingSpinner";
import { EmptyState } from "../components/EmptyState";
import { ErrorBoundary } from "../components/ErrorBoundary";

describe("UI Components", () => {
  describe("LoadingSpinner", () => {
    it("exports a function component", () => {
      expect(typeof LoadingSpinner).toBe("function");
    });

    it("has the expected parameter signature", () => {
      const fn = LoadingSpinner.toString();
      expect(fn).toContain("message");
      expect(fn).toContain("Loading...");
    });
  });

  describe("EmptyState", () => {
    it("exports a function component", () => {
      expect(typeof EmptyState).toBe("function");
    });

    it("accepts icon, title, description, and action props", () => {
      const fn = EmptyState.toString();
      expect(fn).toContain("icon");
      expect(fn).toContain("title");
      expect(fn).toContain("description");
      expect(fn).toContain("action");
    });
  });

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
