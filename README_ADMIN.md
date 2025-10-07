Admin & health endpoints (vision-node)

- /livez (GET)
  - Returns 200 OK with plain `ok` when the server is live.

- /readyz (GET)
  - Returns 200 OK with JSON { "ready": true } when the server is ready to accept requests.

- /admin/ping (GET, POST)
  - Requires an admin token. The token may be provided in one of:
    - Header `x-admin-token: <token>`
    - Query `?token=<token>`
    - Header `Authorization: Bearer <token>`
  - Without a valid token returns 401 with JSON { "error": "invalid or missing admin token" }.
  - With a valid token returns 200 with JSON { "ok": true, "ts": <unix_secs> }.
  - Each successful ping increments the Prometheus counter `vision_admin_ping_total`.

- /admin/info (GET, POST)
  - Protected like `/admin/ping`. Returns JSON with version, git hash and uptime.

- /metrics.prom (GET)
  - Returns plaintext Prometheus metrics including `vision_admin_ping_total`.

Environment variables:
- VISION_ADMIN_TOKEN: admin token value (required for admin endpoints in tests)
- VISION_PORT: port the node listens on (default 7070)
- VISION_VERSION, VISION_GIT_HASH: optional metadata included in /admin/info

Testing:
- The integration tests spawn the built `vision-node` binary and call the endpoints.
  Example: `cargo test --test admin_smoke -- --nocapture`.
