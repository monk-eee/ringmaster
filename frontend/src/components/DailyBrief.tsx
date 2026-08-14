import type { DailyBriefItem } from "../api";
import StatusBadge from "./StatusBadge";
import { typeIcon } from "../icons";

type Props = { items: DailyBriefItem[] };

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
              <span className="type-icon" aria-hidden="true">
                {typeIcon("obligation")}
              </span>
              <StatusBadge value={item.status} />
              <code className="id-marker" title={item.obligation_id}>
                {item.obligation_id.slice(0, 8)}…
              </code>
            </div>
            <span className="daily-brief-reason">{item.reason}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}
