type StatusBadgeProps = { value: string };

const CLASS_BY_VALUE: Record<string, string> = {
  open: "badge badge-open",
  at_risk: "badge badge-at-risk",
  closed: "badge badge-closed",
};

const ICON_BY_VALUE: Record<string, string> = {
  open: "\u25CF", // ●
  at_risk: "\u25B2", // ▲
  closed: "\u2713", // ✓
};

export default function StatusBadge({ value }: StatusBadgeProps) {
  return (
    <span className={CLASS_BY_VALUE[value] ?? "badge"}>
      <span aria-hidden="true">{ICON_BY_VALUE[value] ?? "\u25CF"}</span> {value}
    </span>
  );
}
