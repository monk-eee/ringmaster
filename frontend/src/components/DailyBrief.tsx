import type { DailyBriefItem } from "../api";
import StatusBadge from "./StatusBadge";

type Props = { items: DailyBriefItem[] };

function shortId(id: string): string {
  return `${id.slice(0, 8)}…`;
}

export default function DailyBrief({ items }: Props) {
  if (items.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">Nothing needs attention right now.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <p className="daily-brief-summary">
        {items.length} thing{items.length === 1 ? "" : "s"} need{items.length === 1 ? "s" : ""} attention.
      </p>
      <ol className="daily-brief-list">
        {items.map((item) => (
          <li key={item.obligation_id}>
            <div className="daily-brief-row">
              <code title={item.obligation_id}>{shortId(item.obligation_id)}</code>
              <StatusBadge value={item.status} />
            </div>
            <span className="daily-brief-reason">{item.reason}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}
