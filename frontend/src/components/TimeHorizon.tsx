import type { TimeHorizon, TimeHorizonItem } from "../api";
import StatusBadge from "./StatusBadge";

type Props = { horizon: TimeHorizon };

const BUCKETS: { key: keyof TimeHorizon; label: string }[] = [
  { key: "overdue", label: "Overdue" },
  { key: "next_7_days", label: "Next 7 Days" },
  { key: "next_30_days", label: "Next 30 Days" },
  { key: "next_90_days", label: "Next 90 Days" },
  { key: "beyond", label: "Beyond 90 Days / No Date" },
];

function BucketSection({ label, items }: { label: string; items: TimeHorizonItem[] }) {
  return (
    <div className="card time-horizon-bucket">
      <p className="daily-brief-summary">
        {label} — {items.length} item{items.length === 1 ? "" : "s"}
      </p>
      <ol className="daily-brief-list">
        {items.map((item) => (
          <li key={item.obligation_id}>
            <div className="daily-brief-row">
              <code title={item.obligation_id}>{item.obligation_id.slice(0, 8)}…</code>
              <StatusBadge value={item.status} />
            </div>
            <span className="daily-brief-reason">{item.reason}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}

export default function TimeHorizon({ horizon }: Props) {
  const sections = BUCKETS.map(({ key, label }) => ({ label, items: horizon[key] ?? [] })).filter(
    (section) => section.items.length > 0,
  );

  if (sections.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">Nothing overdue or due within the next 90 days.</p>
      </div>
    );
  }

  return (
    <div className="time-horizon-sections">
      {sections.map((section) => (
        <BucketSection key={section.label} label={section.label} items={section.items} />
      ))}
    </div>
  );
}
