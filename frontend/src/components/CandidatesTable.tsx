import type { Candidate } from "../api";

type Props = { candidates: Candidate[] };

export default function CandidatesTable({ candidates }: Props) {
  if (candidates.length === 0) {
    return (
      <div className="card">
        <p className="empty-state">No candidates yet.</p>
      </div>
    );
  }

  return (
    <div className="card">
      <table>
        <thead>
          <tr>
            <th>Type</th>
            <th>Statement</th>
            <th>Validation state</th>
            <th>Confidence</th>
          </tr>
        </thead>
        <tbody>
          {candidates.map((c) => (
            <tr key={c.candidate_id}>
              <td>{c.candidate_type}</td>
              <td>{c.statement}</td>
              <td>{c.validation_state}</td>
              <td>{Math.round(c.confidence * 100)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
