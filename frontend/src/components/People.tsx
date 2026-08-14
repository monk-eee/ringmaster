import { useEffect, useState } from "react";
import { fetchNodeDetail, fetchNodes, type GraphNode, type NodeDetail } from "../api";
import { renderRelationshipGroup } from "./GraphExplorer";

// ADR-0039: People is a first-class primary destination over data that
// already exists -- GET /api/nodes?node_type=person and GET /api/nodes/:id,
// both already-accepted (ADR-0025) and already-proven. No new route.
export default function People() {
  const [people, setPeople] = useState<GraphNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchNodes("person")
      .then(setPeople)
      .catch((cause) => setError((cause as Error).message))
      .finally(() => setLoading(false));
  }, []);

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

  if (loading) return <p className="empty-state">Loading…</p>;
  if (error) return <p className="error">Could not reach backend: {error}</p>;
  if (people.length === 0) return <p className="empty-state">No people recorded yet.</p>;

  return (
    <ol className="people-list">
      {people.map((person) => (
        <li key={person.id}>
          <button type="button" className="people-card" onClick={() => openPerson(person.id)}>
            <span className="people-card-name">{person.canonical_text}</span>
            {typeof person.attributes.role === "string" && <span className="people-card-role">{person.attributes.role}</span>}
          </button>
        </li>
      ))}
    </ol>
  );
}
