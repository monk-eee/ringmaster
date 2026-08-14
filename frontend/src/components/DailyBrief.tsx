import type { DailyBriefItem } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

// ADR-0039: Today renders a capped slice of the same ranked list, but the
// greeting/summary sentence must still state the true total, and a capped
// list needs an honest "N more" escape hatch to Timeline rather than
// silently truncating.
type Props = { items: DailyBriefItem[]; totalCount?: number; onViewMore?: () => void };

export default function DailyBrief({ items, totalCount, onViewMore }: Props) {
  const total = totalCount ?? items.length;

  if (total === 0) {
    return (
      <div className="card">
        <p className="empty-state">Nothing needs attention right now.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <p className="daily-brief-summary">
        {total} thing{total === 1 ? "" : "s"} need{total === 1 ? "s" : ""} attention.
      </p>
      <ol className="daily-brief-list">
        {items.map((item) => (
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
            {item.risk_signals.length > 0 && (
              <ul className="risk-signals">
                {item.risk_signals.map((signal) => (
                  <li key={signal.signal}>{signal.explanation}</li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ol>
      {total > items.length && onViewMore && (
        <button type="button" className="daily-brief-view-more" onClick={onViewMore}>
          {total - items.length} more in Timeline →
        </button>
      )}
    </div>
  );
}
