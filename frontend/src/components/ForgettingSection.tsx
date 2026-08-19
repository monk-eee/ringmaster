import type { DailyBriefItem } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";
import { itemTitle, duePhrase } from "./DailyBrief";
import { renderBoldSegments } from "../markdown";

type Props = { items: DailyBriefItem[]; onSelect?: (obligationId: string) => void };

const FORGETTING_CAP = 5;

/// ADR-0053: composes the three existing risk signals (ADR-0041/ADR-0046,
/// plus ADR-0054's isolated) into VISION.md's own named "What am I
/// forgetting?" end-state -- no new signal, no combined score, just a
/// capped, ranked filter over data Daily Brief already returns. Ranked by
/// signal count (more flags first), then Daily Brief's own existing
/// urgency order as a tiebreak (the array's natural order, already sorted
/// that way by the API).
export default function ForgettingSection({ items, onSelect }: Props) {
  const flagged = items
    .filter((item) => item.risk_signals.length > 0)
    .sort((a, b) => b.risk_signals.length - a.risk_signals.length)
    .slice(0, FORGETTING_CAP);

  if (flagged.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">Nothing flagged right now.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <ol className="daily-brief-list">
        {flagged.map((item) => {
          const rowContent = (
            <>
              <div className="daily-brief-row">
                <span className="type-icon" aria-hidden="true">
                  {typeIcon("obligation")}
                </span>
                <StatusBadge value={item.status} />
                <span className="daily-brief-reason">{duePhrase(item.hard_due_at, item.soft_due_at)}</span>
              </div>
              <p className="today-item-title">{renderBoldSegments(itemTitle(item))}</p>
            </>
          );
          return (
            <li key={item.obligation_id}>
              {onSelect ? (
                <button type="button" className="daily-brief-row-button" onClick={() => onSelect(item.obligation_id)}>
                  {rowContent}
                </button>
              ) : (
                rowContent
              )}
              <ul className="risk-signals">
                {item.risk_signals.map((signal) => (
                  <li key={signal.signal}>{signal.explanation}</li>
                ))}
              </ul>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
