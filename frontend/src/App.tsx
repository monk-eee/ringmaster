import { useEffect, useMemo, useState } from "react";
import { fetchCandidates, fetchObligations, type Candidate, type Obligation } from "./api";
import ObligationsTable from "./components/ObligationsTable";
import CandidatesTable from "./components/CandidatesTable";

type Tab = "obligations" | "candidates";
type SortKey = "updated_at" | "status";

export default function App() {
  const [tab, setTab] = useState<Tab>("obligations");
  const [obligations, setObligations] = useState<Obligation[]>([]);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [statusFilter, setStatusFilter] = useState("all");
  const [sortKey, setSortKey] = useState<SortKey>("updated_at");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [nextObligations, nextCandidates] = await Promise.all([fetchObligations(), fetchCandidates()]);
      setObligations(nextObligations);
      setCandidates(nextCandidates);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  const statuses = useMemo(() => Array.from(new Set(obligations.map((o) => o.status))).sort(), [obligations]);

  const visibleObligations = useMemo(() => {
    const filtered = statusFilter === "all" ? obligations : obligations.filter((o) => o.status === statusFilter);
    return [...filtered].sort((a, b) =>
      sortKey === "status" ? a.status.localeCompare(b.status) : b.updated_at.localeCompare(a.updated_at),
    );
  }, [obligations, statusFilter, sortKey]);

  return (
    <>
      <header className="app-bar">
        <img className="logo" src="/ringmaster_logo.png" alt="Ringmaster" />
        <div className="page-heading">
          <div className="eyebrow">Ringmaster</div>
          <h1>{tab === "obligations" ? "Obligations" : "Candidates"}</h1>
        </div>
      </header>
      <main>
        <nav className="tabs" role="tablist" aria-label="Views">
          <button
            role="tab"
            aria-selected={tab === "obligations"}
            className={tab === "obligations" ? "tab tab-active" : "tab"}
            onClick={() => setTab("obligations")}
          >
            Obligations
          </button>
          <button
            role="tab"
            aria-selected={tab === "candidates"}
            className={tab === "candidates" ? "tab tab-active" : "tab"}
            onClick={() => setTab("candidates")}
          >
            Candidates
          </button>
        </nav>

        <div className="toolbar">
          {tab === "obligations" && (
            <>
              <label>
                Status
                <select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
                  <option value="all">All</option>
                  {statuses.map((status) => (
                    <option key={status} value={status}>
                      {status}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Sort by
                <select value={sortKey} onChange={(event) => setSortKey(event.target.value as SortKey)}>
                  <option value="updated_at">Last updated</option>
                  <option value="status">Status</option>
                </select>
              </label>
            </>
          )}
          <button onClick={load} disabled={loading}>
            {loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>

        {error && <p className="error">Could not reach backend: {error}</p>}

        {tab === "obligations" ? (
          <ObligationsTable obligations={visibleObligations} />
        ) : (
          <CandidatesTable candidates={candidates} />
        )}
      </main>
    </>
  );
}
