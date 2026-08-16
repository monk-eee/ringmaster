import type { DailyBriefItem } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

// ADR-0039: Today renders a capped slice of the same ranked list, but the
// greeting/summary sentence must still state the true total, and a capped
// list needs an honest "N more" escape hatch to Timeline rather than
// silently truncating.
type Props = { items: DailyBriefItem[]; totalCount?: number; onViewMore?: () => void };

// ADR-0044: a Today row leads with management meaning, never a raw id. The
// title is the evidence quote when one is linked, and an honest status label
// otherwise -- never a fabricated sentence.
function itemTitle(item: DailyBriefItem): string {
  if (item.source_text) return item.source_text;
  if (item.status === "at_risk") return "At-risk obligation";
  if (item.status === "open") return "Open obligation";
  return `${item.status} obligation`;
}

// ADR-0044: a plain human phrase from the effective due date, or an honest
// "no date" state -- never a fabricated date.
function duePhrase(hardDueAt: string | null, softDueAt: string | null): string {
  const effective = hardDueAt ?? softDueAt;
  if (!effective) return "No date recorded";
  const days = Math.round((new Date(effective).getTime() - Date.now()) / 86_400_000);
  if (days < 0) return `${Math.abs(days)} day${Math.abs(days) === 1 ? "" : "s"} overdue`;
  if (days === 0) return "Due today";
  return `Due in ${days} day${days === 1 ? "" : "s"}`;
}

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
              <span className="daily-brief-reason">{duePhrase(item.hard_due_at, item.soft_due_at)}</span>
            </div>
            <p className="today-item-title">{itemTitle(item)}</p>
            <span className="daily-brief-reason">{item.reason}</span>
            {item.risk_signals.length > 0 && (
              <ul className="risk-signals">
                {item.risk_signals.map((signal) => (
                  <li key={signal.signal}>{signal.explanation}</li>
                ))}
              </ul>
            )}
            <span className="daily-brief-reason">
              {item.source_fragment_id ? "Evidence recorded" : "No evidence recorded"}
            </span>
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
