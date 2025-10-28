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

Release builds and VISION_RELEASE
---------------------------------

When producing a release build we recommend setting the environment variable `VISION_RELEASE=1` for two behaviors:

- `start-node.ps1` will prefer the release binary (`target\release\vision-node.exe`) when `VISION_RELEASE=1`.
- The server's static-file fallback uses an exe-adjacent `public/` location in release mode; setting `VISION_RELEASE=1` signals that behavior.

The repository includes `scripts/make-release.ps1` which sets `VISION_RELEASE=1` for the duration of the packaging run, builds the Rust release, builds the `vision-panel` and copies `vision-panel/dist` into `public/`, then produces a zip artifact under `artifacts/`.

If you run the packaged binary directly on a host, set `VISION_RELEASE=1` in the host environment (or start the node via `start-node.ps1` after setting the env var) to use the exe-relative `public/` fallback.
