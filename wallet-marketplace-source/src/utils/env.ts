export const env = {
  NODE_URL: (import.meta as any).env.VITE_VISION_NODE_URL ?? "http://127.0.0.1:7070",
  MARKET_URL: (import.meta as any).env.VITE_VISION_MARKET_URL ?? "http://127.0.0.1:8080",
  FEATURE_DEV_PANEL: ((import.meta as any).env.VITE_FEATURE_DEV_PANEL ?? "false") === "true",
  // Dev-only bypass flags
  WALLET_DEV_BYPASS: ((import.meta as any).env.VITE_WALLET_DEV_BYPASS ?? "0") === "1",
  MOCK_CHAIN: ((import.meta as any).env.VITE_MOCK_CHAIN ?? "0") === "1",
  ELECTRUM_PLAINTEXT: ((import.meta as any).env.VITE_ELECTRUM_PLAINTEXT ?? "0") === "1",
}
