// ADR-0030: one glyph per docs/PRODUCT-SPEC.md SS5.2 node type and candidate_type.
// Shared concepts (risk, decision, request, follow_up) reuse the same glyph
// whether they appear as a graph node or a candidate row. Plain Unicode
// emoji only -- no icon font or image asset dependency.
const TYPE_ICONS: Record<string, string> = {
  person: "\u{1F464}", // 👤
  meeting: "\u{1F4C5}", // 📅
  source_fragment: "\u{1F4AC}", // 💬
  obligation: "\u{1F4CC}", // 📌
  commitment: "\u{1F91D}", // 🤝
  request: "\u{1F64B}", // 🙋
  follow_up: "\u{1F501}", // 🔁
  risk: "\u{26A0}\u{FE0F}", // ⚠️
  decision: "\u{1F9ED}", // 🧭
  expectation: "\u{1F3AF}", // 🎯
  date_event: "\u{1F4C6}", // 📆
  customer_problem: "\u{1F9E9}", // 🧩
  outcome: "\u{1F3C1}", // 🏁
  service: "\u{1F6E0}\u{FE0F}", // 🛠️
  evidence: "\u{1F50D}", // 🔍
};

const FALLBACK_ICON = "\u{25CF}"; // ●, a neutral marker for an unrecognized type

/** Returns one glyph for a node_type or candidate_type; never blank, never throws. */
export function typeIcon(type: string): string {
  return TYPE_ICONS[type] ?? FALLBACK_ICON;
}
