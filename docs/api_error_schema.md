Vision Node: Error Schema and Pagination

1) Canonical Error Response

All endpoints that return an error MUST use this JSON shape and appropriate HTTP status code when possible.

{
  "status": "rejected", // or "ignored", "error"
  "code": 429, // HTTP-like code for quick client branching
  "error": "rate_limited"
}

Notes:
- Use HTTP status codes: 400 for bad request, 401 for unauthorized, 403 for forbidden, 404 for not found, 429 for rate limit, 500 for internal error.
- Include a machine-readable `code` integer and a human `error` string.
- Clients should prefer HTTP status for coarse behavior and `code`/`error` for UI messages.

2) Pagination Cursor

- Endpoints that scan DBs (eg. `/receipts`) should return `{ receipts: [...], next_cursor: "..." }` where `next_cursor` is an opaque base64 token representing the next DB key to seek (e.g., `RCPT_PREFIX + last_seen_hash`).
- Use `db.scan_prefix(prefix)` with `sled::Iter::skip_to()` equivalent by constructing the seek key and resuming iteration.

3) Rate-limit headers

- Endpoints using token bucket should include headers:
  X-RateLimit-Limit: <max-per-window>
  X-RateLimit-Remaining: <remaining>
  X-RateLimit-Reset: <unix-ts-when-window-resets>

4) Admin Auth

- Admin endpoints must require header `X-Vision-Admin-Token` matching env `VISION_ADMIN_TOKEN`.
- Dev endpoints must require `VISION_DEV=true` and header `X-Vision-Dev-Token` matching `VISION_DEV_TOKEN`.

5) Consistent 429 vs internal JSON

- For rate-limited requests prefer returning HTTP 429 with the canonical body. For legacy endpoints that previously returned JSON with 200 and a `status: rejected`, add 429 support while keeping backward-compatible JSON in body.

6) OpenAPI

- `openapi.yaml` includes a minimal API surface to generate clients. Keep it in-sync as you add/rename handlers.
