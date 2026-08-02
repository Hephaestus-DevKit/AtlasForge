import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ApprovalModal } from "../components/ApprovalModal";
import { ErrorBoundary } from "../components/ErrorBoundary";
import type { PermissionRequest } from "../types";

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

describe("ApprovalModal", () => {
  it("shows every isolated verification command and lifecycle expansion", () => {
    const request: PermissionRequest = {
      id: "approval-1",
      jobId: null,
      repoId: "repo-1",
      capability: "fs.write_patch",
      scope: "C:\\repo",
      riskLevel: "high",
      command: null,
      contextHash: "hash",
      details: {
        filePath: "src/main.ts",
        workingTreeClean: true,
        isolatedVerificationCommands: [
          {
            command: "npm test",
            expandedCommand: "pretest: prepare\ntest: vitest\nposttest: cleanup",
            risk: "medium",
          },
        ],
      },
      status: "pending",
      createdAt: "2026-08-02T00:00:00Z",
      expiresAt: "2026-08-02T00:15:00Z",
      decidedAt: null,
    };

    const html = renderToStaticMarkup(createElement(ApprovalModal, {
      requests: [request],
      busy: false,
      onApprove: () => undefined,
      onDeny: () => undefined,
    }));

    expect(html).toContain("npm test");
    expect(html).toContain("pretest: prepare");
    expect(html).toContain("posttest: cleanup");
  });
});
