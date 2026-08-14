import { useEffect, useState } from "react";
import { createEdge, createNode, fetchNodeDetail, fetchNodes, updateNode, type GraphNode, type NodeDetail } from "../api";

const RADIUS = 120;
const CENTER = 160;

function parseAttributes(text: string): Record<string, unknown> | undefined {
  const trimmed = text.trim();
  if (!trimmed) return undefined;
  return JSON.parse(trimmed) as Record<string, unknown>;
}

export default function GraphExplorer() {
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [nodeTypeFilter, setNodeTypeFilter] = useState("all");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [detail, setDetail] = useState<NodeDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newNodeType, setNewNodeType] = useState("");
  const [newCanonicalText, setNewCanonicalText] = useState("");
  const [newAttributesText, setNewAttributesText] = useState("");
  const [creating, setCreating] = useState(false);

  const [enrichAttributesText, setEnrichAttributesText] = useState("");
  const [enriching, setEnriching] = useState(false);

  const [edgeTargetId, setEdgeTargetId] = useState("");
  const [edgeType, setEdgeType] = useState("");
  const [linking, setLinking] = useState(false);

  async function loadNodes() {
    try {
      setNodes(await fetchNodes());
    } catch (cause) {
      setError((cause as Error).message);
    }
  }

  async function loadDetail(id: string) {
    setSelectedNodeId(id);
    setEnrichAttributesText("");
    try {
      setDetail(await fetchNodeDetail(id));
    } catch (cause) {
      setError((cause as Error).message);
      setDetail(null);
    }
  }

  useEffect(() => {
    loadNodes();
  }, []);

  const nodeTypes = Array.from(new Set(nodes.map((node) => node.node_type))).sort();
  const visibleNodes = nodeTypeFilter === "all" ? nodes : nodes.filter((node) => node.node_type === nodeTypeFilter);

  async function handleCreate(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    let attributes: Record<string, unknown> | undefined;
    try {
      attributes = parseAttributes(newAttributesText);
    } catch {
      setError("Attributes must be valid JSON (or left blank).");
      return;
    }
    setCreating(true);
    try {
      const node = await createNode(newNodeType, newCanonicalText, attributes);
      setNewNodeType("");
      setNewCanonicalText("");
      setNewAttributesText("");
      await loadNodes();
      await loadDetail(node.id);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setCreating(false);
    }
  }

  async function handleEnrich(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNodeId) return;
    setError(null);
    let attributes: Record<string, unknown> | undefined;
    try {
      attributes = parseAttributes(enrichAttributesText);
    } catch {
      setError("Attributes must be valid JSON (or left blank).");
      return;
    }
    if (!attributes) return;
    setEnriching(true);
    try {
      await updateNode(selectedNodeId, { attributes });
      setEnrichAttributesText("");
      await loadNodes();
      await loadDetail(selectedNodeId);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setEnriching(false);
    }
  }

  async function handleLink(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNodeId || !edgeTargetId || !edgeType.trim()) return;
    setError(null);
    setLinking(true);
    try {
      await createEdge(selectedNodeId, edgeTargetId, edgeType.trim());
      setEdgeTargetId("");
      setEdgeType("");
      await loadDetail(selectedNodeId);
    } catch (cause) {
      setError((cause as Error).message);
    } finally {
      setLinking(false);
    }
  }

  const neighbors = detail?.neighbors ?? [];

  return (
    <div className="graph-explorer">
      {error && <p className="error">{error}</p>}

      <div className="card">
        <h2>Add a node</h2>
        <form className="toolbar" onSubmit={handleCreate}>
          <input placeholder="Node type (e.g. person, risk)" list="node-type-suggestions" value={newNodeType} onChange={(event) => setNewNodeType(event.target.value)} required />
          <datalist id="node-type-suggestions">
            <option value="person" />
            <option value="meeting" />
            <option value="risk" />
            <option value="decision" />
            <option value="expectation" />
            <option value="customer_problem" />
            <option value="outcome" />
            <option value="service" />
          </datalist>
          <input placeholder="Canonical text (e.g. a name)" value={newCanonicalText} onChange={(event) => setNewCanonicalText(event.target.value)} required />
          <input placeholder='Attributes JSON (optional, e.g. {"role":"manager"})' value={newAttributesText} onChange={(event) => setNewAttributesText(event.target.value)} />
          <button type="submit" disabled={creating || !newNodeType.trim() || !newCanonicalText.trim()}>
            {creating ? "Creating…" : "Create node"}
          </button>
        </form>
      </div>

      <div className="graph-layout">
        <div className="card node-list">
          <label>
            Filter by type
            <select value={nodeTypeFilter} onChange={(event) => setNodeTypeFilter(event.target.value)}>
              <option value="all">All</option>
              {nodeTypes.map((nodeType) => (
                <option key={nodeType} value={nodeType}>
                  {nodeType}
                </option>
              ))}
            </select>
          </label>
          {visibleNodes.length === 0 ? (
            <p className="empty-state">No nodes yet.</p>
          ) : (
            <ul className="node-list-items">
              {visibleNodes.map((node) => (
                <li key={node.id}>
                  <button className={node.id === selectedNodeId ? "node-list-button node-list-button-active" : "node-list-button"} onClick={() => loadDetail(node.id)}>
                    <span className="node-type-tag">{node.node_type}</span> {node.canonical_text}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="card node-detail">
          {!detail ? (
            <p className="empty-state">Select a node to drill in.</p>
          ) : (
            <>
              <h2>
                <span className="node-type-tag">{detail.node_type}</span> {detail.canonical_text}
              </h2>
              <p className="lifecycle-state">Lifecycle: {detail.lifecycle_state}</p>
              <dl className="attributes-list">
                {Object.entries(detail.attributes).map(([key, value]) => (
                  <div key={key}>
                    <dt>{key}</dt>
                    <dd>{JSON.stringify(value)}</dd>
                  </div>
                ))}
              </dl>

              <svg width={CENTER * 2} height={CENTER * 2} className="relationship-view" role="img" aria-label={`Relationships for ${detail.canonical_text}`}>
                {neighbors.map((neighbor, index) => {
                  const angle = (2 * Math.PI * index) / Math.max(neighbors.length, 1) - Math.PI / 2;
                  const x = CENTER + RADIUS * Math.cos(angle);
                  const y = CENTER + RADIUS * Math.sin(angle);
                  const label = neighbor.neighbor?.canonical_text ?? "Obligation";
                  return (
                    <g key={neighbor.edge_id}>
                      <line x1={CENTER} y1={CENTER} x2={x} y2={y} className="relationship-edge" />
                      <text x={(CENTER + x) / 2} y={(CENTER + y) / 2 - 4} className="relationship-edge-label" textAnchor="middle">
                        {neighbor.edge_type}
                      </text>
                      <circle
                        cx={x}
                        cy={y}
                        r={18}
                        className={neighbor.neighbor ? "relationship-node relationship-node-clickable" : "relationship-node"}
                        onClick={neighbor.neighbor ? () => loadDetail(neighbor.neighbor!.id) : undefined}
                      />
                      <text x={x} y={y + 32} textAnchor="middle" className="relationship-node-label">
                        {label.length > 14 ? `${label.slice(0, 14)}…` : label}
                      </text>
                    </g>
                  );
                })}
                <circle cx={CENTER} cy={CENTER} r={22} className="relationship-node relationship-node-center" />
                <text x={CENTER} y={CENTER + 40} textAnchor="middle" className="relationship-node-label">
                  {detail.canonical_text.length > 14 ? `${detail.canonical_text.slice(0, 14)}…` : detail.canonical_text}
                </text>
              </svg>

              <form className="toolbar" onSubmit={handleEnrich}>
                <input placeholder='Enrich attributes JSON (e.g. {"team":"platform"})' value={enrichAttributesText} onChange={(event) => setEnrichAttributesText(event.target.value)} />
                <button type="submit" disabled={enriching || !enrichAttributesText.trim()}>
                  {enriching ? "Enriching…" : "Enrich"}
                </button>
              </form>

              <form className="toolbar" onSubmit={handleLink}>
                <select value={edgeTargetId} onChange={(event) => setEdgeTargetId(event.target.value)}>
                  <option value="">Link to…</option>
                  {nodes.filter((node) => node.id !== selectedNodeId).map((node) => (
                    <option key={node.id} value={node.id}>
                      {node.node_type}: {node.canonical_text}
                    </option>
                  ))}
                </select>
                <input placeholder="Relationship (e.g. made, owns)" value={edgeType} onChange={(event) => setEdgeType(event.target.value)} />
                <button type="submit" disabled={linking || !edgeTargetId || !edgeType.trim()}>
                  {linking ? "Linking…" : "Add relationship"}
                </button>
              </form>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
