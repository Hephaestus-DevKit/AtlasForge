import { useEffect, useState } from "react";
import { searchIndex, reindexRepo, listRepositories, listDocuments } from "../api/ipc";
import type { Repository, SearchResult, IndexedDocument, IndexStats } from "../types";
import { BookOpen, Search, RefreshCw, FileText, Filter, ChevronDown, ChevronRight, Folder, Code, FileCode } from "lucide-react";
import { LoadingSpinner } from "../components/LoadingSpinner";
import { EmptyState } from "../components/EmptyState";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function chunkTypeIcon(type: string) {
  switch (type) {
    case "code": return <Code size={12} />;
    case "config": return <FileCode size={12} />;
    default: return <FileText size={12} />;
  }
}

function langBadge(lang: string | null): string | null {
  if (!lang) return null;
  const colors: Record<string, string> = {
    rust: "#dea584", typescript: "#3178c6", javascript: "#f1e05a", python: "#3572a5",
    go: "#00add8", java: "#b07219", ruby: "#701516", shell: "#89e051",
    markdown: "#083fa1", json: "#292929", yaml: "#cb171e", toml: "#9c4221",
  };
  return colors[lang] ?? null;
}

export function Knowledge() {
  const [repos, setRepos] = useState<Repository[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexingRepo, setIndexingRepo] = useState<string | null>(null);
  const [indexStats, setIndexStats] = useState<IndexStats | null>(null);
  const [documents, setDocuments] = useState<IndexedDocument[]>([]);
  const [loadingDocs, setLoadingDocs] = useState(false);
  const [expandedDocs, setExpandedDocs] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadRepos();
  }, []);

  async function loadRepos() {
    try {
      const list = await listRepositories();
      setRepos(list);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load repos");
    }
  }

  useEffect(() => {
    if (selectedRepoId) {
      loadDocuments(selectedRepoId);
    } else {
      setDocuments([]);
    }
  }, [selectedRepoId]);

  async function loadDocuments(repoId: string) {
    try {
      setLoadingDocs(true);
      setDocuments(await listDocuments(repoId));
    } catch (e: any) {
      // non-critical — just log
      console.warn("Failed to load documents:", e);
    } finally {
      setLoadingDocs(false);
    }
  }

  async function handleSearch() {
    if (!query.trim()) return;
    try {
      setSearching(true);
      setError(null);
      setResults(await searchIndex(query, 20, selectedRepoId ?? undefined));
    } catch (e: any) {
      setError(e?.toString() ?? "Search failed");
    } finally {
      setSearching(false);
    }
  }

  async function handleReindex(repoId: string) {
    try {
      setIndexingRepo(repoId);
      setIndexStats(null);
      setError(null);
      const stats = await reindexRepo(repoId);
      setIndexStats(stats);
      // Refresh documents if this repo is selected
      if (selectedRepoId === repoId) {
        await loadDocuments(repoId);
      }
    } catch (e: any) {
      setError(e?.toString() ?? "Indexing failed");
    } finally {
      setIndexingRepo(null);
    }
  }

  const selectedRepo = repos.find((r) => r.id === selectedRepoId);

  return (
    <div>
      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>Knowledge</h1>

      {error && (
        <div style={{ padding: 12, background: "#fef2f2", border: "1px solid #fca5a5", borderRadius: 6, marginBottom: 16, color: "#991b1b" }}>
          {error}
        </div>
      )}

      {indexStats && (
        <div style={{ padding: 12, background: "#f0fdf4", border: "1px solid #bbf7d0", borderRadius: 6, marginBottom: 16, color: "#166534", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span>
            Indexed <strong>{indexStats.documents}</strong> documents, <strong>{indexStats.chunks}</strong> chunks
            {indexStats.errors.length > 0 && ` (${indexStats.errors.length} errors)`}
          </span>
          <button onClick={() => setIndexStats(null)} style={{ background: "none", border: "none", cursor: "pointer", color: "#166534", fontSize: 18, lineHeight: 1 }}>×</button>
        </div>
      )}

      {/* Search */}
      <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0", marginBottom: 16 }}>
        <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
          <Search size={16} /> Code Search
        </h2>

        {/* Repo filter */}
        <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
          <Filter size={14} style={{ color: "#64748b" }} />
          <select
            value={selectedRepoId ?? ""}
            onChange={(e) => setSelectedRepoId(e.target.value || null)}
            style={{
              flex: 1, padding: "8px 12px", border: "1px solid #e2e8f0", borderRadius: 6,
              fontSize: 13, background: "#fff", color: "#334155", outline: "none",
            }}
          >
            <option value="">All repositories</option>
            {repos.map((repo) => (
              <option key={repo.id} value={repo.id}>
                {repo.worktreePath.split(/[/\\]/).pop()}
              </option>
            ))}
          </select>
        </div>

        <div style={{ display: "flex", gap: 8 }}>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="Search code, configs, docs..."
            style={{
              flex: 1, padding: "10px 14px", border: "1px solid #e2e8f0", borderRadius: 6,
              fontSize: 14, outline: "none",
            }}
          />
          <button
            onClick={handleSearch}
            disabled={searching || !query.trim()}
            style={{
              padding: "10px 20px", background: "#3b82f6", color: "#fff", border: "none",
              borderRadius: 6, cursor: "pointer", fontSize: 14, fontWeight: 600,
              opacity: searching || !query.trim() ? 0.5 : 1,
            }}
          >
            {searching ? "Searching..." : "Search"}
          </button>
        </div>

        {results.length > 0 && (
          <div style={{ marginTop: 16 }}>
            <div style={{ fontSize: 12, color: "#64748b", marginBottom: 8 }}>
              {results.length} result{results.length !== 1 ? "s" : ""} found
              {selectedRepo ? ` in ${selectedRepo.worktreePath.split(/[/\\]/).pop()}` : ""}
            </div>
            {results.map((r, i) => (
              <div key={i} style={{ padding: 10, background: "#f8fafc", borderRadius: 6, marginBottom: 8, border: "1px solid #e2e8f0" }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4, flexWrap: "wrap", gap: 4 }}>
                  <span style={{ fontSize: 13, fontWeight: 600, display: "flex", alignItems: "center", gap: 4 }}>
                    {chunkTypeIcon(r.chunkType)} {r.path}
                    {r.heading && <span style={{ fontWeight: 400, color: "#64748b" }}> § {r.heading}</span>}
                  </span>
                  <div style={{ display: "flex", gap: 8, alignItems: "center", fontSize: 11, color: "#94a3b8" }}>
                    {!selectedRepoId && (
                      <span style={{ display: "flex", alignItems: "center", gap: 2 }}>
                        <Folder size={10} /> {repos.find((rp) => rp.id === r.repoId)?.worktreePath.split(/[/\\]/).pop() ?? r.repoId.slice(0, 8)}
                      </span>
                    )}
                    {r.startLine != null && <span>L{r.startLine}{r.endLine != null ? `–L${r.endLine}` : ""}</span>}
                    <span>Score: {r.rank.toFixed(3)}</span>
                  </div>
                </div>
                <pre style={{ margin: 0, padding: "8px 10px", background: "#1e293b", color: "#e2e8f0", borderRadius: 4, fontSize: 12, overflow: "auto", maxHeight: 120 }}>
                  {r.content.length > 500 ? r.content.slice(0, 500) + "..." : r.content}
                </pre>
              </div>
            ))}
          </div>
        )}

        {searching && <LoadingSpinner message="Searching..." />}

        {results.length === 0 && query && !searching && (
          <p style={{ color: "#94a3b8", fontSize: 13, marginTop: 12 }}>No results. Try a different query or index repositories first.</p>
        )}
      </div>

      {/* Index Management */}
      <div style={{ background: "#fff", borderRadius: 8, padding: 20, border: "1px solid #e2e8f0" }}>
        <h2 style={{ fontSize: 16, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
          <BookOpen size={16} /> Index Management
        </h2>
        {repos.length === 0 ? (
          <EmptyState
            icon={BookOpen}
            title="No repositories found"
            description="Scan workspace roots first to enable indexing."
          />
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
            <thead>
              <tr style={{ borderBottom: "1px solid #e2e8f0" }}>
                <th style={{ textAlign: "left", padding: "8px 0", color: "#64748b" }}>Repository</th>
                <th style={{ textAlign: "left", padding: "8px 0", color: "#64748b" }}>Path</th>
                <th style={{ textAlign: "left", padding: "8px 0", color: "#64748b" }}>Docs</th>
                <th style={{ textAlign: "right", padding: "8px 0", color: "#64748b" }}>Action</th>
              </tr>
            </thead>
            <tbody>
              {repos.map((repo) => (
                <tr key={repo.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "8px 0" }}>
                    <div style={{ fontWeight: 500 }}>{repo.worktreePath.split(/[/\\]/).pop()}</div>
                  </td>
                  <td style={{ padding: "8px 0", fontFamily: "monospace", fontSize: 12, color: "#64748b" }}>
                    {repo.worktreePath}
                  </td>
                  <td style={{ padding: "8px 0" }}>
                    {selectedRepoId === repo.id && documents.length > 0 ? (
                      <span style={{ fontSize: 12, color: "#0369a1" }}>{documents.length} files</span>
                    ) : (
                      <span style={{ fontSize: 12, color: "#94a3b8" }}>—</span>
                    )}
                  </td>
                  <td style={{ padding: "8px 0", textAlign: "right" }}>
                    <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                      <button
                        onClick={() => setSelectedRepoId(selectedRepoId === repo.id ? null : repo.id)}
                        style={{
                          display: "inline-flex", alignItems: "center", gap: 4,
                          padding: "4px 10px", background: selectedRepoId === repo.id ? "#eff6ff" : "#f8fafc",
                          color: selectedRepoId === repo.id ? "#1d4ed8" : "#64748b",
                          border: `1px solid ${selectedRepoId === repo.id ? "#93c5fd" : "#e2e8f0"}`,
                          borderRadius: 4, cursor: "pointer", fontSize: 12,
                        }}
                      >
                        <FileText size={12} />
                        {selectedRepoId === repo.id ? "Selected" : "Browse"}
                      </button>
                      <button
                        onClick={() => handleReindex(repo.id)}
                        disabled={indexingRepo === repo.id}
                        style={{
                          display: "inline-flex", alignItems: "center", gap: 4,
                          padding: "4px 10px", background: "#f0f9ff", color: "#0369a1",
                          border: "1px solid #bae6fd", borderRadius: 4, cursor: "pointer", fontSize: 12,
                        }}
                      >
                        <RefreshCw size={12} />
                        {indexingRepo === repo.id ? "Indexing..." : "Re-index"}
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {/* Document browser for selected repo */}
        {selectedRepoId && (
          <div style={{ marginTop: 16, borderTop: "1px solid #e2e8f0", paddingTop: 16 }}>
            <button
              onClick={() => setExpandedDocs(!expandedDocs)}
              style={{
                display: "flex", alignItems: "center", gap: 6, background: "none", border: "none",
                cursor: "pointer", fontSize: 14, fontWeight: 600, color: "#334155", padding: 0,
              }}
            >
              {expandedDocs ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              Indexed Documents
              {loadingDocs && <span style={{ fontSize: 12, fontWeight: 400, color: "#64748b" }}>Loading...</span>}
              {!loadingDocs && <span style={{ fontSize: 12, fontWeight: 400, color: "#64748b" }}>({documents.length})</span>}
            </button>

            {expandedDocs && documents.length > 0 && (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, marginTop: 8 }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid #e2e8f0" }}>
                    <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Path</th>
                    <th style={{ textAlign: "left", padding: "6px 0", color: "#64748b" }}>Language</th>
                    <th style={{ textAlign: "right", padding: "6px 0", color: "#64748b" }}>Size</th>
                    <th style={{ textAlign: "right", padding: "6px 0", color: "#64748b" }}>Indexed</th>
                  </tr>
                </thead>
                <tbody>
                  {documents.map((doc) => (
                    <tr key={doc.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                      <td style={{ padding: "4px 0", fontFamily: "monospace", fontSize: 11 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                          {chunkTypeIcon(doc.mimeType.includes("markdown") ? "text" : doc.mimeType.includes("json") || doc.mimeType.includes("yaml") ? "config" : "code")}
                          {doc.path}
                        </div>
                      </td>
                      <td style={{ padding: "4px 0" }}>
                        {doc.language && (
                          <span style={{
                            padding: "1px 6px", borderRadius: 3, fontSize: 10, fontWeight: 600,
                            background: langBadge(doc.language) ? `${langBadge(doc.language)}20` : "#f1f5f9",
                            color: langBadge(doc.language) ?? "#64748b",
                          }}>
                            {doc.language}
                          </span>
                        )}
                      </td>
                      <td style={{ padding: "4px 0", textAlign: "right", color: "#64748b" }}>{formatBytes(doc.sizeBytes)}</td>
                      <td style={{ padding: "4px 0", textAlign: "right", color: "#94a3b8" }}>{doc.indexedAt.replace("T", " ").slice(0, 19)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}

            {expandedDocs && documents.length === 0 && !loadingDocs && (
              <p style={{ color: "#94a3b8", fontSize: 12, marginTop: 8 }}>
                No indexed documents. Click "Re-index" to index this repository.
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
