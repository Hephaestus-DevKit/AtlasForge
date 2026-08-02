import type { AiProvider, Artifact, ContextPreview } from "../../../types";
import { AlertTriangle, Eye, Wand2 } from "lucide-react";
import { EmptyText } from "./tabUi";
// --- Fixes Tab ---

export function FixesTab({ fixPlans, aiProviders, selectedProviderId, onSelectProvider,
  generatingPlan, onGeneratePlan, proposingFix, onProposeFix,
  fixInstruction, onFixInstructionChange, fixTargetFile, onFixTargetFileChange,
  hasSnapshot, contextPreview, previewingContext, onPreviewContext }: {
  fixPlans: Artifact[]; aiProviders: AiProvider[]; selectedProviderId: string;
  onSelectProvider: (id: string) => void; generatingPlan: boolean;
  onGeneratePlan: () => void; proposingFix: boolean; onProposeFix: () => void;
  fixInstruction: string; onFixInstructionChange: (v: string) => void;
  fixTargetFile: string; onFixTargetFileChange: (v: string) => void;
  hasSnapshot: boolean;
  contextPreview: ContextPreview | null;
  previewingContext: boolean;
  onPreviewContext: () => void;
}) {
  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <h3 style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)", marginBottom: 8 }}>
          <Wand2 size={14} style={{ verticalAlign: "middle", marginRight: 6 }} />
          AI Fix Assistant
        </h3>

        {/* Provider selector */}
        {/* Provider selector */}
        <div style={{ marginBottom: 12 }}>
          <label htmlFor="af-ai-provider" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
            AI Provider
          </label>
          {aiProviders.length === 0 ? (
            <p style={{ fontSize: 11, color: "var(--text-secondary)", fontStyle: "italic" }}>
              No AI providers configured. Add API keys in Settings.
            </p>
          ) : (
            <select
              id="af-ai-provider"
              value={selectedProviderId}
              onChange={(e) => onSelectProvider(e.target.value)}
              style={{ fontSize: 12, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", background: "var(--bg-input)", color: "var(--text-primary)" }}
            >
              <option value="">Select provider...</option>
              {aiProviders.map((p) => (
                <option key={p.id} value={p.id}>{p.name} ({p.adapterType})</option>
              ))}
            </select>
          )}
        </div>

        {/* Generate Fix Plan */}
        <div style={{ marginBottom: 12, padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Generate Fix Plan</h4>
          <p style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
            Analyze audit findings and generate a prioritized fix plan using AI.
          </p>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <button
              onClick={onGeneratePlan}
              disabled={!selectedProviderId || !hasSnapshot || generatingPlan}
              style={{
                display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
                background: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "var(--border-color)" : "var(--color-accent)",
                color: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "var(--text-secondary)" : "#fff",
                border: "none", borderRadius: 4, cursor: (!selectedProviderId || !hasSnapshot || generatingPlan) ? "not-allowed" : "pointer",
                fontSize: 12, fontWeight: 600,
              }}
            >
              <Wand2 size={12} />
              {generatingPlan ? "Generating..." : "Generate Fix Plan"}
            </button>
            <button
              onClick={onPreviewContext}
              disabled={!hasSnapshot || previewingContext}
              style={{
                display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
                background: (!hasSnapshot || previewingContext) ? "var(--border-color)" : "var(--color-primary)",
                color: (!hasSnapshot || previewingContext) ? "var(--text-secondary)" : "#fff",
                border: "none", borderRadius: 4, cursor: (!hasSnapshot || previewingContext) ? "not-allowed" : "pointer",
                fontSize: 12, fontWeight: 600,
              }}
            >
              <Eye size={12} />
              {previewingContext ? "Loading..." : "Preview Context"}
            </button>
          </div>
          {!hasSnapshot && (
            <p style={{ fontSize: 10, color: "var(--color-warning-text)", marginTop: 4 }}>Run a health audit first to enable fix plan generation.</p>
          )}
        </div>

        {/* Context Preview */}
        {contextPreview && (
          <div style={{ marginBottom: 12, padding: 12, background: "var(--color-info-bg)", borderRadius: 6, border: "1px solid var(--color-info-border)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
              <Eye size={14} color="var(--color-info-text)" />
              <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--color-info-text)", margin: 0 }}>Context Preview</h4>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>
              <strong>Purpose:</strong> {contextPreview.purpose}
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>
              <strong>Tokens:</strong> ~{contextPreview.totalTokensEstimate} / {contextPreview.maxTokens} max
            </div>
            {contextPreview.secretsFound.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 6, padding: "4px 8px", background: "var(--color-warning-bg)", borderRadius: 4, border: "1px solid var(--color-warning-border)" }}>
                <AlertTriangle size={12} color="var(--color-warning-text)" />
                <span style={{ fontSize: 11, color: "var(--color-warning-text)", fontWeight: 600 }}>
                  {contextPreview.secretsFound.length} secret(s) detected — will be redacted before sending to AI
                </span>
              </div>
            )}
            {contextPreview.secretCountAfterRedaction > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 6, padding: "4px 8px", background: "var(--color-danger-bg)", borderRadius: 4, border: "1px solid var(--color-danger-border)" }}>
                <AlertTriangle size={12} color="var(--color-danger-text)" />
                <span style={{ fontSize: 11, color: "var(--color-danger-text)", fontWeight: 600 }}>
                  {contextPreview.secretCountAfterRedaction} secret(s) remain after redaction — review before proceeding
                </span>
              </div>
            )}
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4, fontWeight: 600 }}>Sections:</div>
            {contextPreview.sections.map((s, i) => (
              <div key={i} style={{ padding: "6px 8px", marginBottom: 4, background: "var(--bg-input)", borderRadius: 4, border: "1px solid var(--border-color)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2 }}>
                  <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-primary)" }}>{s.label}</span>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>~{s.tokensEstimate} tokens</span>
                </div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 2 }}>Source: {s.source}</div>
                <pre style={{ fontSize: 10, color: "var(--text-secondary)", background: "var(--bg-app)", padding: 4, borderRadius: 3, overflow: "auto", maxHeight: 80, whiteSpace: "pre-wrap", margin: 0 }}>
                  {s.contentPreview}
                </pre>
              </div>
            ))}
            {contextPreview.promptPreview && (
              <details style={{ marginTop: 8 }}>
                <summary style={{ fontSize: 11, color: "var(--color-primary)", cursor: "pointer", fontWeight: 600 }}>Full Prompt Preview</summary>
                <pre style={{ fontSize: 10, color: "var(--text-secondary)", background: "var(--bg-input)", padding: 8, borderRadius: 4, overflow: "auto", maxHeight: 300, whiteSpace: "pre-wrap", marginTop: 4 }}>
                  {contextPreview.promptPreview}
                </pre>
              </details>
            )}
          </div>
        )}

        {/* Propose Fix */}
        <div style={{ marginBottom: 12, padding: 12, background: "var(--bg-app)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
          <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 6 }}>Propose a Fix</h4>
          <div style={{ marginBottom: 8 }}>
            <label htmlFor="af-fix-instruction" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Instruction
            </label>
            <textarea
              id="af-fix-instruction"
              value={fixInstruction}
              onChange={(e) => onFixInstructionChange(e.target.value)}
              placeholder="Describe what you want to fix (e.g., 'Fix the missing return type in src/main.ts')"
              rows={3}
              style={{ fontSize: 12, padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", resize: "vertical", fontFamily: "inherit", background: "var(--bg-input)", color: "var(--text-primary)" }}
            />
          </div>
          <div style={{ marginBottom: 8 }}>
            <label htmlFor="af-fix-target" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Target File (optional)
            </label>
            <input
              id="af-fix-target"
              value={fixTargetFile}
              onChange={(e) => onFixTargetFileChange(e.target.value)}
              placeholder="e.g., src/main.ts"
              style={{ fontSize: 12, padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", width: "100%", background: "var(--bg-input)", color: "var(--text-primary)" }}
            />
          </div>
          <button
            onClick={onProposeFix}
            disabled={!selectedProviderId || !fixInstruction.trim() || proposingFix}
            style={{
              display: "flex", alignItems: "center", gap: 4, padding: "6px 12px",
              background: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "var(--border-color)" : "var(--color-accent)",
              color: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "var(--text-secondary)" : "#fff",
              border: "none", borderRadius: 4, cursor: (!selectedProviderId || !fixInstruction.trim() || proposingFix) ? "not-allowed" : "pointer",
              fontSize: 12, fontWeight: 600,
            }}
          >
            <Wand2 size={12} />
            {proposingFix ? "Proposing..." : "Propose Fix"}
          </button>
        </div>
      </div>

      {/* Fix Plans list */}
      <div>
        <h4 style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)", marginBottom: 8 }}>Fix Plans</h4>
        {fixPlans.length === 0 ? (
          <EmptyText>No fix plans generated yet.</EmptyText>
        ) : (
          fixPlans.map((fp) => (
            <div key={fp.id} style={{ padding: 10, marginBottom: 8, background: "var(--bg-input)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                <Wand2 size={12} color="var(--color-accent)" />
                <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{fp.artifactType}: {fp.title}</span>
              </div>
              {fp.content && (
                <pre style={{ fontSize: 11, color: "var(--text-secondary)", background: "var(--bg-app)", padding: 8, borderRadius: 4, overflow: "auto", maxHeight: 200, whiteSpace: "pre-wrap" }}>
                  {typeof fp.content === "string" ? fp.content : JSON.stringify(fp.content, null, 2)}
                </pre>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
