import express from "express";

const PORT = process.env.PORT || 3000;
const BACKEND_URL = process.env.BACKEND_URL || "http://localhost:8080";

const app = express();

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

// ADR-0012: server-rendered only; no client-side JavaScript or bundler yet.
app.get("/", async (_request, response) => {
  let obligations = [];
  let error = null;
  try {
    const backendResponse = await fetch(`${BACKEND_URL}/api/obligations`);
    if (!backendResponse.ok) throw new Error(`backend responded ${backendResponse.status}`);
    obligations = await backendResponse.json();
  } catch (cause) {
    error = cause.message;
  }

  const rows = obligations
    .map(
      (o) =>
        `<tr><td>${escapeHtml(o.obligation_id)}</td><td>${escapeHtml(o.status)}</td><td>${escapeHtml(o.updated_at)}</td></tr>`,
    )
    .join("\n");

  response.type("html").send(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Ringmaster — Obligations</title>
</head>
<body>
  <h1>Obligations</h1>
  ${error ? `<p class="error">Could not reach backend: ${escapeHtml(error)}</p>` : ""}
  <table>
    <thead><tr><th>Obligation</th><th>Status</th><th>Updated</th></tr></thead>
    <tbody>
${rows}
    </tbody>
  </table>
</body>
</html>`);
});

app.listen(PORT, () => {
  console.log(`ringmaster-frontend: listening on :${PORT}, backend=${BACKEND_URL}`);
});
