import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  applyPatch,
  decidePermissionRequest,
  listRepositorySummaries,
  requestPatchApproval,
  requestVerificationApproval,
  runBatchVerification,
  runVerification,
} from "../api/ipc";
import type { PermissionRequest, VerificationCommand } from "../types";

beforeAll(() => {
  if (!("window" in globalThis)) {
    Object.assign(globalThis, { window: globalThis });
  }
});

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

function approval(id: string): PermissionRequest {
  return {
    id,
    jobId: null,
    repoId: "repo-1",
    capability: "shell.verify",
    scope: "C:\\repo",
    riskLevel: "medium",
    command: "npm test",
    contextHash: "hash",
    details: {},
    status: "pending",
    createdAt: "2026-06-20T00:00:00Z",
    expiresAt: "2026-06-20T00:15:00Z",
    decidedAt: null,
  };
}

describe("Tauri IPC contracts", () => {
  it("loads repository summaries in one command", async () => {
    const calls = vi.fn();
    mockIPC((command, payload) => {
      calls(command, payload);
      if (command === "list_repository_summaries") return [];
      throw new Error(`Unexpected command: ${command}`);
    });

    await expect(listRepositorySummaries()).resolves.toEqual([]);
    expect(calls).toHaveBeenCalledWith("list_repository_summaries", {});
  });

  it("prepares and decides a context-bound verification approval", async () => {
    const calls: Array<[string, Record<string, unknown> | undefined]> = [];
    mockIPC((command, payload) => {
      calls.push([command, payload as Record<string, unknown> | undefined]);
      if (command === "request_verification_approval_cmd") return approval("approval-1");
      if (command === "decide_permission_request_cmd") {
        return { ...approval("approval-1"), status: "approved" };
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    await requestVerificationApproval("repo-1", "C:\\repo", "npm test");
    await decidePermissionRequest("approval-1", true);
    expect(calls).toEqual([
      [
        "request_verification_approval_cmd",
        { repoId: "repo-1", cwd: "C:\\repo", command: "npm test" },
      ],
      [
        "decide_permission_request_cmd",
        { requestId: "approval-1", approved: true },
      ],
    ]);
  });

  it("passes the consumed approval into verification", async () => {
    let received: Record<string, unknown> | undefined;
    mockIPC((command, payload) => {
      expect(command).toBe("run_verification_cmd");
      received = payload as Record<string, unknown>;
      return {
        success: true,
        command: "npm test",
        exitCode: 0,
        stdout: "",
        stderr: "",
        durationMs: 10,
        timedOut: false,
      };
    });

    await runVerification("npm test", "C:\\repo", "repo-1", "approval-1");
    expect(received).toEqual({
      command: "npm test",
      cwd: "C:\\repo",
      repoId: "repo-1",
      approvalId: "approval-1",
    });
  });

  it("keeps batch command and approval ordering aligned", async () => {
    const commands: VerificationCommand[] = [
      {
        name: "npm test",
        command: "npm test",
        timeoutSecs: 120,
        category: "test",
        riskLevel: "medium",
        requiresApproval: true,
        expandedCommand: "test: vitest",
        riskExplanation: "Repository script",
      },
      {
        name: "npm run lint",
        command: "npm run lint",
        timeoutSecs: 60,
        category: "lint",
        riskLevel: "medium",
        requiresApproval: true,
        expandedCommand: "lint: eslint .",
        riskExplanation: "Repository script",
      },
    ];
    let received: Record<string, unknown> | undefined;
    mockIPC((command, payload) => {
      expect(command).toBe("run_batch_verification_cmd");
      received = payload as Record<string, unknown>;
      return [];
    });

    await runBatchVerification(
      commands,
      "C:\\repo",
      "repo-1",
      ["approval-test", "approval-lint"],
    );
    expect(received).toEqual({
      commandNames: ["npm test", "npm run lint"],
      cwd: "C:\\repo",
      repoId: "repo-1",
      approvalIds: ["approval-test", "approval-lint"],
    });
  });

  it("requires patch approval before applying", async () => {
    const calls: Array<[string, Record<string, unknown> | undefined]> = [];
    mockIPC((command, payload) => {
      calls.push([command, payload as Record<string, unknown> | undefined]);
      if (command === "request_patch_approval_cmd") {
        return { ...approval("patch-approval"), capability: "fs.write_patch" };
      }
      if (command === "apply_patch_cmd") {
        return {
          id: "patch-1",
          jobId: "job-1",
          artifactId: null,
          repoId: "repo-1",
          filePath: "src/main.ts",
          patchContent: "",
          description: "fix",
          status: "applied",
          appliedAt: "2026-06-20T00:00:00Z",
          rolledBackAt: null,
          verificationResult: null,
        };
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    await requestPatchApproval("patch-1");
    await applyPatch("patch-1", "patch-approval");
    expect(calls[1]).toEqual([
      "apply_patch_cmd",
      { proposalId: "patch-1", approvalId: "patch-approval" },
    ]);
  });
});
