import { useEffect, useState } from "react";
import { fetchNodeDetail, fetchNodes, type NodeDetail, type PersonListNode } from "../api";
import { renderRelationshipGroup } from "./GraphExplorer";

function relativeInteraction(lastInteractionAt: string | null): string {
  if (!lastInteractionAt) return "No recorded interaction";
  const days = Math.floor((Date.now() - new Date(lastInteractionAt).getTime()) / 86_400_000);
  if (days <= 0) return "Last heard from today";
  if (days === 1) return "Last heard from yesterday";
  return `Last heard from ${days} days ago`;
}

// ADR-0039: People is a first-class primary destination over data that
// already exists -- GET /api/nodes?node_type=person and GET /api/nodes/:id,
// both already-accepted (ADR-0025) and already-proven. No new route.
// ADR-0051: defaults to who needs something from you, not every person
// node -- an explicit, honest toggle switches back to everyone.
export default function People() {
  const [people, setPeople] = useState<PersonListNode[]>([]);
  const [needsAttentionOnly, setNeedsAttentionOnly] = useState(true);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchNodes("person", needsAttentionOnly)
      .then((nodes) => setPeople(nodes as PersonListNode[]))
      .catch((cause) => setError((cause as Error).message))
      .finally(() => setLoading(false));
  }, [needsAttentionOnly]);

  async function openPerson(id: string) {
    setSelectedId(id);
    setDetail(null);
    setDetailError(null);
    try {
      setDetail(await fetchNodeDetail(id));
    } catch (cause) {
      setDetailError((cause as Error).message);
    }
  }

  if (selectedId) {
    return (
      <div className="people-detail">
        <button type="button" className="people-back" onClick={() => setSelectedId(null)}>
          ← All people
        </button>
        {detailError && <p className="error">{detailError}</p>}
        {!detailError && !detail && <p className="empty-state">Loading…</p>}
        {detail && (
          <div className="card">
            <h3>{detail.canonical_text}</h3>
            <p className="people-card-interaction">{relativeInteraction(detail.last_interaction_at)}</p>
            <dl className="attributes-list">
              {Object.entries(detail.attributes).map(([key, value]) => (
                <div key={key}>
                  <dt>{key}</dt>
                  <dd>{JSON.stringify(value)}</dd>
                </div>
              ))}
            </dl>
            <div className="relationship-obligations">
              <h4>Relationship</h4>
              {!detail.relationship || (detail.relationship.at_risk.length === 0 && detail.relationship.open.length === 0) ? (
                <p className="empty-state">Nothing owed either way yet.</p>
              ) : (
                <>
                  {renderRelationshipGroup("At Risk", detail.relationship.at_risk)}
                  {renderRelationshipGroup("Open Commitments", detail.relationship.open)}
                </>
              )}
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <>
      <div className="toolbar">
        <button type="button" className="people-attention-toggle" onClick={() => setNeedsAttentionOnly((value) => !value)}>
          {needsAttentionOnly ? "Show everyone" : "Show only who needs attention"}
        </button>
      </div>
      {loading ? (
        <p className="empty-state">Loading…</p>
      ) : error ? (
        <p className="error">Could not reach backend: {error}</p>
      ) : people.length === 0 ? (
        <p className="empty-state">{needsAttentionOnly ? "Nobody currently needs anything from you." : "No people recorded yet."}</p>
      ) : (
        <ol className="people-list">
          {people.map((person) => (
            <li key={person.id}>
              <button type="button" className="people-card" onClick={() => openPerson(person.id)}>
                <span className="people-card-name">{person.canonical_text}</span>
                {typeof person.attributes.role === "string" && <span className="people-card-role">{person.attributes.role}</span>}
                {(person.at_risk_count > 0 || person.open_count > 0) && (
                  <span className="people-card-owed">
                    {person.at_risk_count > 0 && `${person.at_risk_count} at risk`}
                    {person.at_risk_count > 0 && person.open_count > 0 && ", "}
                    {person.open_count > 0 && `${person.open_count} open`}
                  </span>
                )}
                <span className="people-card-interaction">{relativeInteraction(person.last_interaction_at)}</span>
              </button>
            </li>
          ))}
        </ol>
      )}
    </>
  );
}
