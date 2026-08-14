import { useEffect, useMemo, useState } from "react";
import {
  fetchCandidates,
  fetchDailyBrief,
  fetchObligations,
  searchSourceFragments,
  type Candidate,
  type DailyBriefItem,
  type Obligation,
  type SearchResult,
} from "./api";
import ObligationsTable from "./components/ObligationsTable";
import CandidatesTable from "./components/CandidatesTable";
import SearchResults from "./components/SearchResults";
import DailyBrief from "./components/DailyBrief";

type Tab = "daily-brief" | "obligations" | "candidates" | "search";
type SortKey = "updated_at" | "status";

const TAB_TITLES: Record<Tab, string> = {
  "daily-brief": "Daily Brief",
  obligations: "Obligations",
  candidates: "Candidates",
  search: "Search",
};

export default function App() {
  const [tab, setTab] = useState<Tab>("daily-brief");
  const [obligations, setObligations] = useState<Obligation[]>([]);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [dailyBrief, setDailyBrief] = useState<DailyBriefItem[]>([]);
  const [statusFilter, setStatusFilter] = useState("all");
  const [sortKey, setSortKey] = useState<SortKey>("updated_at");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [hasSearched, setHasSearched] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [nextObligations, nextCandidates, nextDailyBrief] = await Promise.all([
        fetchObligations(),
        fetchCandidates(),
        fetchDailyBrief(),
      ]);
      setObligations(nextObligations);
      setCandidates(nextCandidates);
      setDailyBrief(nextDailyBrief);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setLoading(false);
    }
  }

  async function runSearch(event: React.FormEvent) {
    event.preventDefault();
    if (!searchQuery.trim()) return;
    setSearchLoading(true);
    setSearchError(null);
    try {
      setSearchResults(await searchSourceFragments(searchQuery));
    } catch (cause) {
      setSearchError((cause as Error).message);
      setSearchResults([]);
    } finally {
      setHasSearched(true);
      setSearchLoading(false);
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
        <img className="logo" src="/ringmaster_logo.png?v=2" alt="Ringmaster" />
        <div className="page-heading">
          <div className="eyebrow">Ringmaster</div>
          <h1>{TAB_TITLES[tab]}</h1>
        </div>
      </header>
      <main>
        <nav className="tabs" role="tablist" aria-label="Views">
          <button
            role="tab"
            aria-selected={tab === "daily-brief"}
            className={tab === "daily-brief" ? "tab tab-active" : "tab"}
            onClick={() => setTab("daily-brief")}
          >
            Daily Brief
          </button>
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
          <button
            role="tab"
            aria-selected={tab === "search"}
            className={tab === "search" ? "tab tab-active" : "tab"}
            onClick={() => setTab("search")}
          >
            Search
          </button>
        </nav>

        {tab === "search" ? (
          <>
            <form className="toolbar" onSubmit={runSearch}>
              <input
                type="search"
                placeholder="Search meeting evidence…"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
              />
              <button type="submit" disabled={searchLoading || !searchQuery.trim()}>
                {searchLoading ? "Searching…" : "Search"}
              </button>
            </form>
            {searchError && <p className="error">{searchError}</p>}
            <SearchResults results={searchResults} hasSearched={hasSearched} />
          </>
        ) : tab === "daily-brief" ? (
          <>
            <div className="toolbar">
              <button onClick={load} disabled={loading}>
                {loading ? "Refreshing…" : "Refresh"}
              </button>
            </div>
            {error && <p className="error">Could not reach backend: {error}</p>}
            <DailyBrief items={dailyBrief} />
          </>
        ) : (
          <>
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
              <CandidatesTable candidates={candidates} onChanged={load} />
            )}
          </>
        )}
      </main>
    </>
  );
}
