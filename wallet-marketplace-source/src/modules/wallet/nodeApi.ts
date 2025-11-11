import { loadConfig } from "../../lib/config"

async function getJSON<T>(path: string): Promise<T> {
  const { apiBase } = await loadConfig();
  const r = await fetch(`${apiBase}${path}`)
  if (!r.ok) throw new Error(`${path} ${r.status}`)
  return r.json()
}

export async function getStatus() { return getJSON<any>("/api/status") }
export async function getSupply() { return getJSON<any>("/api/supply") }
export async function getVault()  { return getJSON<any>("/api/wallet/info") }
export async function getLatestReceipts() { return getJSON<any>("/api/receipts/latest") }
