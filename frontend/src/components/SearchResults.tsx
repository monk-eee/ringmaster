import type { SearchResult } from "../api";

type Props = { results: SearchResult[]; hasSearched: boolean };

export default function SearchResults({ results, hasSearched }: Props) {
  if (!hasSearched) {
    return (
      <div className="card">
        <p className="empty-state">Search meeting evidence by meaning, not just keywords.</p>
      </div>
    );
  }

  if (results.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">No matching source fragments.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <table>
        <thead>
          <tr>
            <th>Match</th>
            <th>Speaker</th>
            <th>Similarity</th>
          </tr>
        </thead>
        <tbody>
          {results.map((r) => (
            <tr key={r.source_fragment_id}>
              <td>"{r.text}"</td>
              <td>{r.speaker ?? "unknown"}</td>
              <td>{Math.round(r.similarity * 100)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
