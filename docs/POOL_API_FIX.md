# Pool API 405 Error Fix

## Issue
When trying to start the mining pool from the panel UI, users encountered:
```
❌ Error starting pool: HTTP 405
```

## Root Cause
All API routes in Vision Node are nested under `/api/*` prefix (configured in `src/main.rs` at line ~6210):

```rust
Router::new()
    .nest("/api", api)  // All API routes are under /api/*
    .nest_service("/app", wallet_with_no_cache)
    .route("/panel", get(|| async { Redirect::permanent("/panel.html") }))
    .route("/", get(|| async { Redirect::permanent("/app") }))
    .fallback_service(static_service)
```

However, the frontend (`public/panel.html`) was calling pool endpoints without the `/api` prefix:
```javascript
// ❌ WRONG - 405 Method Not Allowed
fetch('/pool/start', { method: 'POST', ... })

// ✅ CORRECT - Routes to API handler
fetch('/api/pool/start', { method: 'POST', ... })
```

## Solution
Updated all pool API calls in `public/panel.html` to use the `/api` prefix:

```bash
# Applied regex replacement
(Get-Content "public\panel.html") -replace "fetch\('/pool/", "fetch('/api/pool/" | Set-Content "public\panel.html"
```

### Affected Endpoints
- `/pool/mode` → `/api/pool/mode` (GET and POST)
- `/pool/start` → `/api/pool/start` (POST)
- `/pool/stop` → `/api/pool/stop` (POST)
- `/pool/stats` → `/api/pool/stats` (GET)

## Testing
```powershell
# Test pool start
Invoke-WebRequest -Uri "http://localhost:7070/api/pool/start" `
    -Method POST `
    -ContentType "application/json" `
    -Body '{"pool_fee": 1.5, "pool_name": "Test Pool", "pool_port": 7072}' `
    -UseBasicParsing

# Expected response:
# StatusCode: 200
# Content: {"ok":true,"message":"Pool hosting started","mode":"host_pool",...}
```

## Files Changed
1. `public/panel.html` - Updated all pool API fetch calls
2. `release-package/public/panel.html` - Synced with same changes

## Prevention
All future API endpoints should follow the convention:
- **Frontend**: Always use `/api/*` prefix for API calls
- **Backend**: Routes are registered in `build_app()` and automatically nested under `/api`

## Related Files
- `src/main.rs` (line 5965-5974) - Pool route definitions
- `src/pool/routes.rs` - Pool endpoint handlers
- `public/panel.html` - Mining panel UI

## Status
✅ **FIXED** - Pool can now be started/stopped/configured from panel UI without 405 errors.
