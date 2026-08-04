import { createElement } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ApprovalModal } from "../components/ApprovalModal";
import { ErrorBoundary } from "../components/ErrorBoundary";
import type { PermissionRequest } from "../types";

afterEach(cleanup);

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

  it("shows every isolated verification command and lifecycle expansion", () => {
    render(createElement(ApprovalModal, {
      requests: [request],
      busy: false,
      onApprove: () => undefined,
      onDeny: () => undefined,
    }));
    expect(screen.getByText("npm test")).toBeTruthy();
    expect(screen.getByText(/pretest: prepare/)).toBeTruthy();
    expect(screen.getByText(/posttest: cleanup/)).toBeTruthy();
  });

  it("captures initial focus, closes on Escape, and restores focus", () => {
    const trigger = document.createElement("button");
    document.body.appendChild(trigger);
    trigger.focus();
    const onDeny = vi.fn();
    const view = render(createElement(ApprovalModal, {
      requests: [request],
      busy: false,
      onApprove: () => undefined,
      onDeny,
    }));
    expect(screen.getByRole("dialog")).toBe(document.activeElement);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onDeny).toHaveBeenCalledTimes(1);
    view.unmount();
    expect(trigger).toBe(document.activeElement);
    trigger.remove();
  });
});
