type StatusBadgeProps = { value: string };

const CLASS_BY_VALUE: Record<string, string> = {
  open: "badge badge-open",
  at_risk: "badge badge-at-risk",
  closed: "badge badge-closed",
};

export default function StatusBadge({ value }: StatusBadgeProps) {
  return <span className={CLASS_BY_VALUE[value] ?? "badge"}>{value}</span>;
}
