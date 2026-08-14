import type { TimeHorizon, TimeHorizonItem } from "../api";
import StatusBadge from "./StatusBadge";

type Props = { horizon: TimeHorizon };

const BUCKETS: { key: keyof TimeHorizon; label: string; accent: string }[] = [
  { key: "overdue", label: "Overdue", accent: "overdue" },
  { key: "next_7_days", label: "Next 7 Days", accent: "next-7" },
  { key: "next_30_days", label: "Next 30 Days", accent: "next-30" },
  { key: "next_90_days", label: "Next 90 Days", accent: "next-90" },
  { key: "beyond", label: "Beyond 90 Days / No Date", accent: "beyond" },
];

function BucketSection({ label, accent, items }: { label: string; accent: string; items: TimeHorizonItem[] }) {
  return (
    <div className={`card time-horizon-bucket accent-${accent}`}>
      <div className="time-horizon-bucket-header">
        <span className="time-horizon-dot" aria-hidden="true" />
        <p className="time-horizon-title">{label}</p>
        <span className="time-horizon-count">{items.length}</span>
      </div>
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
  const sections = BUCKETS.map(({ key, label, accent }) => ({ label, accent, items: horizon[key] ?? [] })).filter(
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
    <div className="time-horizon">
      <div className="time-horizon-ribbon">
        {sections.map((section) => (
          <span key={section.label} className={`time-horizon-chip accent-${section.accent}`}>
            {section.label} <strong>{section.items.length}</strong>
          </span>
        ))}
      </div>
      <div className="time-horizon-sections">
        {sections.map((section) => (
          <BucketSection key={section.label} label={section.label} accent={section.accent} items={section.items} />
        ))}
      </div>
    </div>
  );
}
