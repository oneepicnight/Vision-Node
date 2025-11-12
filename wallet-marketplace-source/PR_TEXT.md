feat(market): canonical cash store + migration, admin paging, tests, CI

- Sled tree `market_cash_orders`; migration from legacy keys
- Cursor pagination for /admin/cash/orders (base64 "<updated_at>|<id>")
- Tests: cursor_roundtrip, cash_store_migration, cash_admin_cursor
- Dev feature gating for test helpers; crate-level clippy gates
- GitHub Actions: Ubuntu + Windows; Windows tests single-thread
