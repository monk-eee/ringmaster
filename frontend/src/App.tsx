import { useEffect, useMemo, useState } from "react";
import {
  fetchCandidates,
  fetchDailyBrief,
  fetchFocusBlocks,
  fetchObligations,
  fetchTimeHorizon,
  searchSourceFragments,
  type Candidate,
  type DailyBriefItem,
  type FocusBlock,
  type Obligation,
  type SearchResult,
  type TimeHorizon as TimeHorizonData,
} from "./api";
import ObligationsTable from "./components/ObligationsTable";
import CandidatesTable from "./components/CandidatesTable";
import SearchResults from "./components/SearchResults";
import DailyBrief from "./components/DailyBrief";
import FocusBlocks from "./components/FocusBlocks";
import GraphExplorer from "./components/GraphExplorer";
import TimeHorizon from "./components/TimeHorizon";
import People from "./components/People";
import ComingSoonStrip from "./components/ComingSoonStrip";
import MeetingReview from "./components/MeetingReview";

// ADR-0039: four primary destinations answer a manager's actual questions
// (what needs attention, what's coming, who do I owe, what's awaiting a
// decision) instead of naming backend entities. Obligations/Search/Graph
// remain fully functional, unchanged developer/diagnostic surfaces --
// demoted in the tab bar, not deleted.
type Tab = "today" | "timeline" | "people" | "inbox" | "obligations" | "search" | "graph" | "meetings";
type SortKey = "updated_at" | "status";

const TAB_TITLES: Record<Tab, string> = {
  today: "Today",
  timeline: "Timeline",
  people: "People",
  inbox: "Inbox",
  obligations: "Obligations",
  search: "Search",
  graph: "Graph",
  meetings: "Meetings",
};

const PRIMARY_TABS: Tab[] = ["today", "timeline", "people", "inbox"];
const SECONDARY_TABS: Tab[] = ["obligations", "search", "graph", "meetings"];
const TODAY_ITEM_CAP = 10;

export default function App() {
  const [tab, setTab] = useState<Tab>("today");
  const [obligations, setObligations] = useState<Obligation[]>([]);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [dailyBrief, setDailyBrief] = useState<DailyBriefItem[]>([]);
  const [focusBlocks, setFocusBlocks] = useState<FocusBlock[]>([]);
  const [timeHorizon, setTimeHorizon] = useState<TimeHorizonData>({});
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
      const [nextObligations, nextCandidates, nextDailyBrief, nextFocusBlocks, nextTimeHorizon] = await Promise.all([
        fetchObligations(),
        fetchCandidates(),
        fetchDailyBrief(),
        fetchFocusBlocks(),
        fetchTimeHorizon(),
      ]);
      setObligations(nextObligations);
      setCandidates(nextCandidates);
      setDailyBrief(nextDailyBrief);
      setFocusBlocks(nextFocusBlocks);
      setTimeHorizon(nextTimeHorizon);
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
          {PRIMARY_TABS.map((primaryTab) => (
            <button
              key={primaryTab}
              role="tab"
              aria-selected={tab === primaryTab}
              className={tab === primaryTab ? "tab tab-active" : "tab"}
              onClick={() => setTab(primaryTab)}
            >
              {TAB_TITLES[primaryTab]}
            </button>
          ))}
          <span className="tabs-divider" aria-hidden="true" />
          <span className="tabs-secondary-label">Developer</span>
          {SECONDARY_TABS.map((secondaryTab) => (
            <button
              key={secondaryTab}
              role="tab"
              aria-selected={tab === secondaryTab}
              className={tab === secondaryTab ? "tab tab-secondary tab-active" : "tab tab-secondary"}
              onClick={() => setTab(secondaryTab)}
            >
              {TAB_TITLES[secondaryTab]}
            </button>
          ))}
        </nav>

        {tab === "graph" ? (
          <GraphExplorer />
        ) : tab === "people" ? (
          <People />
        ) : tab === "meetings" ? (
          <MeetingReview />
        ) : tab === "timeline" ? (
          <>
            <div className="toolbar">
              <button onClick={load} disabled={loading}>
                {loading ? "Refreshing…" : "Refresh"}
              </button>
            </div>
            {error && <p className="error">Could not reach backend: {error}</p>}
            <TimeHorizon horizon={timeHorizon} />
          </>
        ) : tab === "search" ? (
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
        ) : tab === "today" ? (
          <>
            <div className="toolbar">
              <button onClick={load} disabled={loading}>
                {loading ? "Refreshing…" : "Refresh"}
              </button>
            </div>
            {error && <p className="error">Could not reach backend: {error}</p>}
            <p className="today-greeting">
              {dailyBrief.length === 0
                ? "Nothing needs your attention right now."
                : `${dailyBrief.length} thing${dailyBrief.length === 1 ? "" : "s"} need${dailyBrief.length === 1 ? "s" : ""} your attention today.`}
            </p>
            <DailyBrief items={dailyBrief.slice(0, TODAY_ITEM_CAP)} totalCount={dailyBrief.length} onViewMore={() => setTab("timeline")} />
            {focusBlocks.length > 0 && (
              <>
                <h2 className="today-section-heading">Do these together</h2>
                <FocusBlocks blocks={focusBlocks} />
              </>
            )}
            <ComingSoonStrip horizon={timeHorizon} onOpenTimeline={() => setTab("timeline")} />
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
