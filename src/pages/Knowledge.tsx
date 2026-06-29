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
    case "code": return <Code size={14} color="var(--color-primary)" />;
    case "config": return <FileCode size={14} color="var(--color-warning)" />;
    default: return <FileText size={14} color="var(--color-info)" />;
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
      <h1 style={{ fontSize: 26, fontWeight: 800, marginBottom: 24, letterSpacing: "-0.025em" }}>Knowledge</h1>

      {error && (
        <div className="badge badge-danger" style={{ display: "block", width: "100%", padding: 12, borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          {error}
        </div>
      )}

      {indexStats && (
        <div className="badge badge-success" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", padding: "12px 16px", borderRadius: "var(--radius-sm)", marginBottom: 20, fontSize: 13 }}>
          <span>
            Indexed <strong>{indexStats.documents}</strong> documents, <strong>{indexStats.chunks}</strong> chunks
            {indexStats.errors.length > 0 && ` (${indexStats.errors.length} errors)`}
          </span>
          <button onClick={() => setIndexStats(null)} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--color-success-text)", fontSize: 18, lineHeight: 1, padding: 0 }}>×</button>
        </div>
      )}

      {/* Search */}
      <div className="card" style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
          <Search size={18} color="var(--color-primary)" /> Code Search
        </h2>

        {/* Repo filter */}
        <div style={{ display: "flex", gap: 10, marginBottom: 16, alignItems: "center" }}>
          <Filter size={14} color="var(--text-secondary)" />
          <select
            value={selectedRepoId ?? ""}
            onChange={(e) => setSelectedRepoId(e.target.value || null)}
            className="select-field"
            style={{ flex: 1 }}
          >
            <option value="">All repositories</option>
            {repos.map((repo) => (
              <option key={repo.id} value={repo.id}>
                {repo.worktreePath.split(/[/\\]/).pop()}
              </option>
            ))}
          </select>
        </div>

        <div style={{ display: "flex", gap: 10 }}>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="Search code, configs, docs..."
            className="input-field"
            style={{ flex: 1, padding: "12px 16px" }}
          />
          <button
            onClick={handleSearch}
            disabled={searching || !query.trim()}
            className="btn btn-primary"
            style={{
              padding: "12px 24px",
              opacity: searching || !query.trim() ? 0.5 : 1,
            }}
          >
            {searching ? "Searching..." : "Search"}
          </button>
        </div>

        {results.length > 0 && (
          <div style={{ marginTop: 24 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12, fontWeight: 600 }}>
              {results.length} result{results.length !== 1 ? "s" : ""} found
              {selectedRepo ? ` in ${selectedRepo.worktreePath.split(/[/\\]/).pop()}` : ""}
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {results.map((r, i) => (
                <div key={i} style={{ padding: 14, background: "rgba(255, 255, 255, 0.01)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8, flexWrap: "wrap", gap: 10, alignItems: "center" }}>
                    <span style={{ fontSize: 13, fontWeight: 700, display: "flex", alignItems: "center", gap: 6, color: "var(--text-primary)" }}>
                      {chunkTypeIcon(r.chunkType)} {r.path}
                      {r.heading && <span style={{ fontWeight: 400, color: "var(--text-muted)", fontSize: 12 }}> § {r.heading}</span>}
                    </span>
                    <div style={{ display: "flex", gap: 10, alignItems: "center", fontSize: 11, color: "var(--text-muted)" }}>
                      {!selectedRepoId && (
                        <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                          <Folder size={11} /> {repos.find((rp) => rp.id === r.repoId)?.worktreePath.split(/[/\\]/).pop() ?? r.repoId.slice(0, 8)}
                        </span>
                      )}
                      {r.startLine != null && <span style={{ fontFamily: "var(--font-mono)" }}>L{r.startLine}{r.endLine != null ? `–L${r.endLine}` : ""}</span>}
                      <span className="badge badge-neutral" style={{ fontSize: 10 }}>Score: {r.rank.toFixed(3)}</span>
                    </div>
                  </div>
                  <pre style={{ margin: 0, padding: 12, background: "var(--bg-input)", border: "1px solid var(--border-color)", color: "#93c5fd", borderRadius: "var(--radius-sm)", fontSize: 12, fontFamily: "var(--font-mono)", overflow: "auto", maxHeight: 150, whiteSpace: "pre-wrap" }}>
                    {r.content.length > 500 ? r.content.slice(0, 500) + "..." : r.content}
                  </pre>
                </div>
              ))}
            </div>
          </div>
        )}

        {searching && <LoadingSpinner message="Searching index..." />}

        {results.length === 0 && query && !searching && (
          <p style={{ color: "var(--text-secondary)", fontSize: 13, marginTop: 16, fontStyle: "italic" }}>No results. Try a different query or index repositories first.</p>
        )}
      </div>

      {/* Index Management */}
      <div className="card">
        <h2 style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, display: "flex", alignItems: "center", gap: 8 }}>
          <BookOpen size={18} color="var(--color-accent)" /> Index Management
        </h2>
        {repos.length === 0 ? (
          <EmptyState
            icon={BookOpen}
            title="No repositories found"
            description="Scan workspace roots first to enable indexing."
          />
        ) : (
          <div style={{ overflowX: "auto" }}>
            <table className="custom-table" style={{ fontSize: 13 }}>
              <thead>
                <tr>
                  <th>Repository</th>
                  <th>Path</th>
                  <th>Docs</th>
                  <th style={{ textAlign: "right" }}>Action</th>
                </tr>
              </thead>
              <tbody>
                {repos.map((repo) => (
                  <tr key={repo.id} className="table-row-interactive">
                    <td style={{ padding: "12px 16px" }}>
                      <div style={{ fontWeight: 700, color: "var(--text-primary)" }}>{repo.worktreePath.split(/[/\\]/).pop()}</div>
                    </td>
                    <td style={{ padding: "12px 16px", fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>
                      {repo.worktreePath}
                    </td>
                    <td style={{ padding: "12px 16px" }}>
                      {selectedRepoId === repo.id && documents.length > 0 ? (
                        <span className="badge badge-info">{documents.length} files</span>
                      ) : (
                        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>—</span>
                      )}
                    </td>
                    <td style={{ padding: "12px 16px", textAlign: "right" }}>
                      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                        <button
                          onClick={() => setSelectedRepoId(selectedRepoId === repo.id ? null : repo.id)}
                          className={selectedRepoId === repo.id ? "btn btn-primary" : "btn btn-secondary"}
                          style={{ padding: "6px 12px", fontSize: 12 }}
                        >
                          <FileText size={12} />
                          {selectedRepoId === repo.id ? "Selected" : "Browse"}
                        </button>
                        <button
                          onClick={() => handleReindex(repo.id)}
                          disabled={indexingRepo === repo.id}
                          className="btn btn-secondary"
                          style={{ padding: "6px 12px", fontSize: 12, display: "flex", alignItems: "center", gap: 4 }}
                        >
                          <RefreshCw size={12} className={indexingRepo === repo.id ? "spin-slow" : ""} />
                          {indexingRepo === repo.id ? "Indexing..." : "Re-index"}
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Document browser for selected repo */}
        {selectedRepoId && (
          <div style={{ marginTop: 24, borderTop: "1px solid var(--border-color)", paddingTop: 20 }}>
            <button
              onClick={() => setExpandedDocs(!expandedDocs)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                background: "none",
                border: "none",
                cursor: "pointer",
                fontSize: 14,
                fontWeight: 700,
                color: "var(--text-primary)",
                padding: 0,
                outline: "none",
              }}
            >
              {expandedDocs ? <ChevronDown size={18} /> : <ChevronRight size={18} />}
              Indexed Documents
              {loadingDocs && <span style={{ fontSize: 12, fontWeight: 400, color: "var(--text-muted)", marginLeft: 6 }}>Loading...</span>}
              {!loadingDocs && <span style={{ fontSize: 12, fontWeight: 400, color: "var(--text-muted)", marginLeft: 6 }}>({documents.length})</span>}
            </button>

            {expandedDocs && documents.length > 0 && (
              <div style={{ overflowX: "auto", marginTop: 12 }}>
                <table className="custom-table" style={{ fontSize: 12 }}>
                  <thead>
                    <tr>
                      <th>Path</th>
                      <th>Language</th>
                      <th style={{ textAlign: "right" }}>Size</th>
                      <th style={{ textAlign: "right" }}>Indexed</th>
                    </tr>
                  </thead>
                  <tbody>
                    {documents.map((doc) => (
                      <tr key={doc.id} className="table-row-interactive">
                        <td style={{ padding: "8px 16px", fontFamily: "var(--font-mono)", fontSize: 11 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            {chunkTypeIcon(doc.mimeType.includes("markdown") ? "text" : doc.mimeType.includes("json") || doc.mimeType.includes("yaml") ? "config" : "code")}
                            {doc.path}
                          </div>
                        </td>
                        <td style={{ padding: "8px 16px" }}>
                          {doc.language && (
                            <span style={{
                              padding: "2px 6px", borderRadius: 4, fontSize: 10, fontWeight: 600,
                              background: langBadge(doc.language) ? `${langBadge(doc.language)}20` : "rgba(255,255,255,0.05)",
                              color: langBadge(doc.language) ?? "var(--text-secondary)",
                              border: langBadge(doc.language) ? `1px solid ${langBadge(doc.language)}30` : "1px solid var(--border-color)",
                            }}>
                              {doc.language}
                            </span>
                          )}
                        </td>
                        <td style={{ padding: "8px 16px", textAlign: "right", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{formatBytes(doc.sizeBytes)}</td>
                        <td style={{ padding: "8px 16px", textAlign: "right", color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>{doc.indexedAt.replace("T", " ").slice(0, 19)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {expandedDocs && documents.length === 0 && !loadingDocs && (
              <p style={{ color: "var(--text-secondary)", fontSize: 12, marginTop: 12, fontStyle: "italic" }}>
                No indexed documents. Click "Re-index" to index this repository.
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
