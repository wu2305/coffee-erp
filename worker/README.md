# Coffee ERP Worker

Cloudflare Worker API for the shared Coffee ERP state document.

## API

- `GET /api/state?store_id=...`
  - Reads `coffee_erp:store:{store_id}:state` from KV.
  - Returns the seed AppState when KV has no document yet.
- `PUT /api/state?store_id=...`
  - Accepts a complete AppState JSON document.
  - Requires the submitted `revision` to match the current KV revision.
  - Persists the document with `revision + 1`.
  - Returns `409 revision_conflict` when the submitted revision is stale.

## Environment

- `KV_NAMESPACE`: Cloudflare KV binding.
- `ALLOWED_ORIGINS`: comma-separated CORS allowlist.
- `STORE_ID_SEED`: optional default store ID when the query parameter is absent.

## Local Tests

```bash
pnpm test
```
