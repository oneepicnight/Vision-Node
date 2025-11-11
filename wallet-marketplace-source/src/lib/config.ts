// src/lib/config.ts
// Runtime configuration loader - no Vite env dependencies

export type WalletConfig = {
  apiBase?: string;
  wsBase?: string;
  chain?: string;
};

let CFG: Required<WalletConfig> | null = null;

export async function loadConfig(): Promise<Required<WalletConfig>> {
  if (CFG) return CFG;
  
  try {
    const res = await fetch('/app/wallet-config.json', { cache: 'no-store' });
    const c: WalletConfig = res.ok ? await res.json() : {};
    CFG = {
      apiBase: (c.apiBase ?? '').replace(/\/$/, '') || `${location.protocol}//${location.host}`,
      wsBase: (c.wsBase ?? '').replace(/\/$/, '') || `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}`,
      chain: c.chain || 'LAND'
    };
  } catch {
    // Fallback to same-origin if config fetch fails
    CFG = {
      apiBase: `${location.protocol}//${location.host}`,
      wsBase: `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}`,
      chain: 'LAND'
    };
  }
  
  return CFG!;
}

// Helper to get config synchronously after it's been loaded
export function getConfig(): Required<WalletConfig> | null {
  return CFG;
}
