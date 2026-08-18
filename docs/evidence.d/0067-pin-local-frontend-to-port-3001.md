# EV-0067: Pin the local frontend to port 3001

Evidence for [ADR-0067](../adr.d/0067-pin-local-frontend-to-port-3001.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0067-pin-local-frontend-to-port-3001"

[[check]]
id = "vite-pins-strict-port-3001"
invariant = "Vite is pinned to port 3001 and refuses automatic port fallback."
type = "present"
pattern = 'Number\(process\.env\.VITE_PORT\) \|\| 3001[\s\S]*?strictPort: true'
paths = ["frontend/vite.config.ts"]

[[check]]
id = "local-launchers-use-port-3001"
invariant = "Compose and Playwright launch and address the frontend on port 3001."
type = "manual"
last_verified = "2026-08-17"
rationale = "Reviewed the effective local launch configuration: compose.yaml maps 3001:3001, frontend/Dockerfile exposes 3001, and both Playwright webServer.url and use.baseURL point to http://localhost:3001."

[[check]]
id = "frontend-live-on-3001"
invariant = "The active Ringmaster frontend responds successfully on port 3001."
type = "manual"
last_verified = "2026-08-17"
rationale = "After applying the persisted configuration, the unchanged Vite process remained listening on 127.0.0.1:3001 and returned HTTP 200. The frontend production build also completed successfully."
```
