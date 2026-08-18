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
import ObligationDetail from "./components/ObligationDetail";
import FocusBlocks from "./components/FocusBlocks";
import ForgettingSection from "./components/ForgettingSection";
import GraphExplorer from "./components/GraphExplorer";
import TimeHorizon from "./components/TimeHorizon";
import People from "./components/People";
import ComingSoonStrip from "./components/ComingSoonStrip";
import MeetingReview from "./components/MeetingReview";
import Activity from "./components/Activity";

// ADR-0039: four primary destinations answer a manager's actual questions
// (what needs attention, what's coming, who do I owe, what's awaiting a
// decision) instead of naming backend entities. Obligations/Search/Graph
// remain fully functional, unchanged developer/diagnostic surfaces --
// demoted in the tab bar, not deleted.
type Tab = "today" | "timeline" | "people" | "inbox" | "obligations" | "search" | "graph" | "meetings" | "activity";
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
  activity: "Activity",
};

// ADR-0080: Graph is promoted to primary nav -- its progressive traversal
// trail (ADR-0033) now answers a primary management question, not a
// database-browser one. Obligations/Search remain demoted per ADR-0039.
const PRIMARY_TABS: Tab[] = ["today", "timeline", "people", "inbox", "graph"];
const SECONDARY_TABS: Tab[] = ["obligations", "search", "meetings", "activity"];
const TODAY_ITEM_CAP = 10;
// ADR-0059: default page size for the Obligations/Candidates/People list
// views -- a full page back means there may be more; a short page is the end.
const LIST_PAGE_SIZE = 50;

// ADR-0084: honest about what is and isn't known -- no stored display name
// exists anywhere in this app, so the greeting is time-of-day only.
function timeOfDayGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning.";
  if (hour < 18) return "Good afternoon.";
  return "Good evening.";
}

export default function App() {
  const [tab, setTab] = useState<Tab>("today");
  const [selectedObligationId, setSelectedObligationId] = useState<string | null>(null);
  const [obligations, setObligations] = useState<Obligation[]>([]);
  const [obligationsHasMore, setObligationsHasMore] = useState(false);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [candidatesHasMore, setCandidatesHasMore] = useState(false);
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
        fetchObligations(LIST_PAGE_SIZE, 0),
        fetchCandidates(LIST_PAGE_SIZE, 0),
        fetchDailyBrief(),
        fetchFocusBlocks(),
        fetchTimeHorizon(),
      ]);
      setObligations(nextObligations);
      setObligationsHasMore(nextObligations.length === LIST_PAGE_SIZE);
      setCandidates(nextCandidates);
      setCandidatesHasMore(nextCandidates.length === LIST_PAGE_SIZE);
      setDailyBrief(nextDailyBrief);
      setFocusBlocks(nextFocusBlocks);
      setTimeHorizon(nextTimeHorizon);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setLoading(false);
    }
  }

  // ADR-0059: appends the next page rather than re-fetching everything.
  async function loadMoreObligations() {
    const next = await fetchObligations(LIST_PAGE_SIZE, obligations.length);
    setObligations((current) => [...current, ...next]);
    setObligationsHasMore(next.length === LIST_PAGE_SIZE);
  }

  async function loadMoreCandidates() {
    const next = await fetchCandidates(LIST_PAGE_SIZE, candidates.length);
    setCandidates((current) => [...current, ...next]);
    setCandidatesHasMore(next.length === LIST_PAGE_SIZE);
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

  useEffect(() => {
    document.title = `Ringmaster — ${TAB_TITLES[tab]}`;
  }, [tab]);

  const statuses = useMemo(() => Array.from(new Set(obligations.map((o) => o.status))).sort(), [obligations]);

  // ADR-0084: reuses the exact risk_signals already fetched with each daily
  // brief item -- no new signal, no new route, a plain client-side filter.
  const dateCompressionCount = useMemo(
    () => dailyBrief.filter((item) => item.risk_signals.some((signal) => signal.signal === "date_compression")).length,
    [dailyBrief],
  );
  const staleCount = useMemo(
    () => dailyBrief.filter((item) => item.risk_signals.some((signal) => signal.signal === "stale")).length,
    [dailyBrief],
  );

  // ADR-0047: a Today/Obligations row opens the same shared detail view in
  // place of the tab's own list -- switching tabs always leaves it behind.
  function switchTab(nextTab: Tab) {
    setSelectedObligationId(null);
    setTab(nextTab);
  }

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
              onClick={() => switchTab(primaryTab)}
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
              onClick={() => switchTab(secondaryTab)}
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
        ) : tab === "activity" ? (
          <Activity />
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
          selectedObligationId ? (
            <ObligationDetail obligationId={selectedObligationId} onBack={() => setSelectedObligationId(null)} />
          ) : (
          <>
            <div className="toolbar">
              <button onClick={load} disabled={loading}>
                {loading ? "Refreshing…" : "Refresh"}
              </button>
            </div>
            {error && <p className="error">Could not reach backend: {error}</p>}
            {dailyBrief.length === 0 ? (
              <p className="today-greeting">Nothing needs your attention right now.</p>
            ) : (
              <div className="today-summary">
                <p className="today-greeting">{timeOfDayGreeting()}</p>
                <p className="today-summary-line">
                  {dailyBrief.length} thing{dailyBrief.length === 1 ? "" : "s"} need{dailyBrief.length === 1 ? "s" : ""} attention today.
                </p>
                {dateCompressionCount > 0 && (
                  <p className="today-summary-line">
                    {dateCompressionCount} will become risk{dateCompressionCount === 1 ? "" : "s"} this week.
                  </p>
                )}
                {staleCount > 0 && (
                  <p className="today-summary-line">
                    {staleCount} commitment{staleCount === 1 ? "" : "s"} appear{staleCount === 1 ? "s" : ""} forgotten.
                  </p>
                )}
              </div>
            )}
            <DailyBrief
              items={dailyBrief.slice(0, TODAY_ITEM_CAP)}
              totalCount={dailyBrief.length}
              onViewMore={() => switchTab("timeline")}
              onSelect={setSelectedObligationId}
            />
            <h2 className="today-section-heading">What am I forgetting?</h2>
            <ForgettingSection items={dailyBrief} onSelect={setSelectedObligationId} />
            {focusBlocks.length > 0 && (
              <>
                <h2 className="today-section-heading">Do these together</h2>
                <FocusBlocks blocks={focusBlocks} />
              </>
            )}
            <ComingSoonStrip horizon={timeHorizon} onOpenTimeline={() => switchTab("timeline")} />
          </>
          )
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
              selectedObligationId ? (
                <ObligationDetail obligationId={selectedObligationId} onBack={() => setSelectedObligationId(null)} />
              ) : (
                <ObligationsTable obligations={visibleObligations} onSelect={setSelectedObligationId} hasMore={obligationsHasMore} onLoadMore={loadMoreObligations} />
              )
            ) : (
              <CandidatesTable candidates={candidates} onChanged={load} hasMore={candidatesHasMore} onLoadMore={loadMoreCandidates} />
            )}
          </>
        )}
      </main>
    </>
  );
}
