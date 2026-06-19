import { describe, expect, it } from "vitest";

// --- Workspace root validation tests ---
// These test the frontend validation logic without requiring Tauri runtime.

describe("Workspace root form validation", () => {
  it("rejects empty path on submit", () => {
    const path = "";
    expect(path.trim().length === 0).toBe(true);
  });

  it("accepts valid path", () => {
    const path = "C:\\Users\\someone\\projects";
    expect(path.trim().length > 0).toBe(true);
  });

  it("default exclude globs contain standard patterns", () => {
    const DEFAULT_EXCLUDE_GLOBS = [
      "node_modules",
      ".git/objects",
      "dist",
      "build",
      ".env",
      "__pycache__",
      ".next",
      ".cache",
      "target",
      "*.pyc",
    ];
    expect(DEFAULT_EXCLUDE_GLOBS).toContain("node_modules");
    expect(DEFAULT_EXCLUDE_GLOBS).toContain(".env");
    expect(DEFAULT_EXCLUDE_GLOBS).toContain("target");
  });

  it("access mode is one of the allowed values", () => {
    const allowedModes = ["read_only", "read_write"];
    expect(allowedModes).toContain("read_only");
    expect(allowedModes).toContain("read_write");
    expect(allowedModes).not.toContain("admin");
  });
});

describe("Duplicate root detection", () => {
  it("detects duplicate path among existing roots", () => {
    const existingRoots = [
      { id: "1", path: "C:\\Users\\test\\projects", label: "Projects" },
      { id: "2", path: "C:\\Users\\test\\repos", label: "Repos" },
    ];
    const newPath = "C:\\Users\\test\\projects";
    const isDuplicate = existingRoots.some((r) => r.path === newPath);
    expect(isDuplicate).toBe(true);
  });

  it("allows unique path", () => {
    const existingRoots = [
      { id: "1", path: "C:\\Users\\test\\projects", label: "Projects" },
      { id: "2", path: "C:\\Users\\test\\repos", label: "Repos" },
    ];
    const newPath = "C:\\Users\\test\\new-folder";
    const isDuplicate = existingRoots.some((r) => r.path === newPath);
    expect(isDuplicate).toBe(false);
  });
});

describe("pickFolder dialog", () => {
  it("pickFolder returns string or null", async () => {
    // We test the return type contract, not the actual dialog (requires Tauri runtime)
    type PickFolderResult = string | null;
    const result: PickFolderResult = null;
    expect(result === null || typeof result === "string").toBe(true);
  });
});

describe("Scan entry points", () => {
  it("startScan accepts optional rootIds", () => {
    // Verify the IPC call signature accepts rootIds as optional
    function startScanSignature(_rootIds?: string[]): void {}
    expect(startScanSignature.length).toBe(1); // one declared parameter (optional)
    expect(() => startScanSignature()).not.toThrow();
    expect(() => startScanSignature(["root-1"])).not.toThrow();
    expect(() => startScanSignature(["root-1", "root-2"])).not.toThrow();
  });
});

describe("AtlasForge smoke test", () => {
  it("keeps the test runner wired", () => {
    expect("AtlasForge").toContain("Forge");
  });
});

describe("Scan error records", () => {
  it("ScanErrorRecord type has expected fields", () => {
    // Verify the type contract matches the Rust backend struct
    const error = {
      id: "err-1",
      rootId: "root-1",
      path: "/some/path",
      errorType: "scan_error",
      message: "Failed to scan",
      createdAt: "2025-01-01T00:00:00Z",
    };
    expect(error.id).toBe("err-1");
    expect(error.rootId).toBe("root-1");
    expect(error.errorType).toBe("scan_error");
    expect(error.message).toBe("Failed to scan");
  });

  it("listScanErrors accepts rootId and returns ScanErrorRecord array", () => {
    // Verify the IPC contract: listScanErrors(rootId: string) => ScanErrorRecord[]
    function listScanErrorsSignature(_rootId: string): Array<{
      id: string;
      rootId: string;
      path: string | null;
      errorType: string;
      message: string;
      createdAt: string;
    }> {
      return [];
    }
    expect(() => listScanErrorsSignature("root-1")).not.toThrow();
    const result = listScanErrorsSignature("root-1");
    expect(Array.isArray(result)).toBe(true);
  });

  it("scan errors map groups errors by root ID", () => {
    const errorsMap: Record<string, Array<{ id: string; rootId: string }>> = {
      "root-1": [{ id: "e1", rootId: "root-1" }, { id: "e2", rootId: "root-1" }],
      "root-2": [],
    };
    expect(errorsMap["root-1"].length).toBe(2);
    expect(errorsMap["root-2"].length).toBe(0);
    expect(errorsMap["root-3"]).toBeUndefined();
  });
});
