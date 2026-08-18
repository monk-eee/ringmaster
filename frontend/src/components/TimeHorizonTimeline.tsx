import { useMemo, useState } from "react";
import type { TimeHorizon, TimeHorizonItem } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";
import { BUCKETS } from "./TimeHorizon";

type Stack = { dateLabel: string; items: TimeHorizonItem[] };
type Band = (typeof BUCKETS)[number] & { stacks: Stack[] };

function effectiveDateLabel(item: TimeHorizonItem): string {
  const effective = item.hard_due_at ?? item.soft_due_at;
  if (!effective) return "No date recorded";
  return new Date(effective).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

// Groups a band's items by effective due date (or "No date recorded") so
// same-day Obligations render as one marker with a count instead of
// overlapping (ADR-0035).
function groupByEffectiveDate(items: TimeHorizonItem[]): Stack[] {
  const groups = new Map<string, TimeHorizonItem[]>();
  for (const item of items) {
    const key = effectiveDateLabel(item);
    const existing = groups.get(key);
    if (existing) {
      existing.push(item);
    } else {
      groups.set(key, [item]);
    }
  }
  return Array.from(groups.entries()).map(([dateLabel, groupItems]) => ({ dateLabel, items: groupItems }));
}

type Props = { horizon: TimeHorizon };

export default function TimeHorizonTimeline({ horizon }: Props) {
  const [focusIndex, setFocusIndex] = useState(0);
  const [zoomed, setZoomed] = useState(false);
  const [showLegend, setShowLegend] = useState(false);
  const [expandedStack, setExpandedStack] = useState<string | null>(null);

  const bands: Band[] = useMemo(
    () => BUCKETS.map((bucket) => ({ ...bucket, stacks: groupByEffectiveDate(horizon[bucket.key] ?? []) })),
    [horizon],
  );

  function handleNow() {
    setFocusIndex(0);
    setZoomed(false);
    setExpandedStack(null);
  }

  function handlePan(direction: -1 | 1) {
    setFocusIndex((current) => Math.min(Math.max(current + direction, 0), BUCKETS.length - 1));
  }

  function toggleStack(stackKey: string) {
    setExpandedStack((current) => (current === stackKey ? null : stackKey));
  }

  const visibleBands = zoomed ? [bands[focusIndex]] : bands;

  return (
    <div className="time-horizon-timeline">
      <div className="time-horizon-toolbar">
        <button type="button" className="time-horizon-now" onClick={handleNow}>
          Now
        </button>
        <button type="button" className="time-horizon-pan" onClick={() => handlePan(-1)} disabled={focusIndex === 0} aria-label="Focus earlier window">
          ◀
        </button>
        <button type="button" className="time-horizon-pan" onClick={() => handlePan(1)} disabled={focusIndex === BUCKETS.length - 1} aria-label="Focus later window">
          ▶
        </button>
        <button type="button" className="time-horizon-zoom" onClick={() => setZoomed((current) => !current)}>
          {zoomed ? "Zoom out" : "Zoom in"}
        </button>
        <button type="button" className="time-horizon-legend-toggle" onClick={() => setShowLegend((current) => !current)}>
          {showLegend ? "Hide legend" : "Show legend"}
        </button>
      </div>

      {showLegend && (
        <div className="time-horizon-legend">
          {BUCKETS.map((bucket) => (
            <span key={bucket.key} className={`time-horizon-chip accent-${bucket.accent}`}>
              {bucket.label}
            </span>
          ))}
          <span className="time-horizon-chip">
            <span aria-hidden="true">{typeIcon("obligation")}</span> Obligation
          </span>
        </div>
      )}

      <div className="time-horizon-bands">
        {visibleBands.map((band) => {
          const bandIndex = BUCKETS.findIndex((candidate) => candidate.key === band.key);
          const itemCount = band.stacks.reduce((total, stack) => total + stack.items.length, 0);
          return (
            <div
              key={band.key}
              className={
                bandIndex === focusIndex ? `time-horizon-band accent-${band.accent} time-horizon-band-focused` : `time-horizon-band accent-${band.accent}`
              }
            >
              <div className="time-horizon-band-header">
                <span className="time-horizon-band-label">{band.label}</span>
                <span className="time-horizon-count">{itemCount}</span>
              </div>
              {band.stacks.length === 0 ? (
                <p className="time-horizon-band-empty">Nothing in this window.</p>
              ) : (
                <div className="time-horizon-markers">
                  {band.stacks.map((stack) => {
                    const stackKey = `${band.key}-${stack.dateLabel}`;
                    return (
                      <div key={stackKey} className="time-horizon-marker-group">
                        <button type="button" className="time-horizon-marker" onClick={() => toggleStack(stackKey)} aria-expanded={expandedStack === stackKey}>
                          <span aria-hidden="true">{typeIcon("obligation")}</span>
                          <span className="time-horizon-marker-date">{stack.dateLabel}</span>
                          {stack.items.length > 1 && <span className="time-horizon-marker-count">{stack.items.length}</span>}
                        </button>
                        {expandedStack === stackKey && (
                          <ol className="daily-brief-list time-horizon-marker-detail">
                            {stack.items.map((item) => (
                              <li key={item.obligation_id}>
                                <div className="daily-brief-row">
                                  <span className="type-icon" aria-hidden="true">
                                    {typeIcon("obligation")}
                                  </span>
                                  <StatusBadge value={item.status} />
                                  <code className="id-marker" title={item.obligation_id}>
                                    {item.obligation_id.slice(0, 8)}…
                                  </code>
                                </div>
                                <span className="daily-brief-reason">{item.reason}</span>
                                {item.source_occurred_at && (
                                  <span className="time-horizon-source-occurred-at">
                                    Source occurred {new Date(item.source_occurred_at).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}
                                  </span>
                                )}
                              </li>
                            ))}
                          </ol>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
