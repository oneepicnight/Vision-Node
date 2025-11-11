import axios from './axios-wrapper'
import { env } from '../utils/env'
import { loadConfig } from './config'

// Safe fetch handler that checks res.ok before parsing JSON
async function handle(res: Response) {
  if (!res.ok) {
    // Try to read text for better errors (404s return HTML/plain)
    const text = await res.text().catch(() => '');
    const err = new Error(`HTTP ${res.status} ${res.statusText} – ${text.slice(0, 200)}`);
    // @ts-ignore - Surface status for callers
    err.status = res.status;
    throw err;
  }
  const ct = res.headers.get('content-type') || '';
  if (ct.includes('application/json')) return res.json();
  // Non-JSON 204/empty ok:
  if (res.status === 204 || res.headers.get('content-length') === '0') return null;
  // Fallback try json, else text
  try {
    return await res.json();
  } catch {
    return await res.text();
  }
}

export async function api(path: string, init?: RequestInit) {
  const { apiBase } = await loadConfig();
  const p = path.startsWith('/') ? path : `/${path}`;
  return handle(await fetch(`${apiBase}${p}`, init));
}

// Convenience wrappers for fetch-based API
export const get = (p: string) => api(p);
export const post = (p: string, body: any) =>
  api(p, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });

export async function wsUrl(path: string) {
  const { wsBase } = await loadConfig();
  const p = path.startsWith('/') ? path : `/${path}`;
  return `${wsBase}${p}`;
}

// Configure axios with default timeout (legacy, keeping for backward compatibility)
const axiosClient = axios.create({
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json'
  }
})

// Base URL management (legacy)
export async function getBaseUrl(): Promise<string> {
  // When running in mock/dev-bypass mode, point to the local dev server
  if (env.MOCK_CHAIN || env.WALLET_DEV_BYPASS) {
    return ''
  }

  // Use runtime config
  const { apiBase } = await loadConfig();
  return localStorage.getItem('vision.node.url') || apiBase
}

export function setBaseUrl(url: string): void {
  localStorage.setItem('vision.node.url', url)
}

// API endpoints
export async function pingStatus(): Promise<{ up: boolean; info: any }> {
  // Dev bypass: short-circuit network checks when developer explicitly requests it
  if (env.WALLET_DEV_BYPASS) {
    return { up: false, info: { devBypass: true } }
  }

  // Mock chain: report node up with mock info
  if (env.MOCK_CHAIN) {
    return { up: true, info: { mock: true } }
  }

  try {
    const baseUrl = await getBaseUrl();
    const response = await axiosClient.get(`${baseUrl}/api/status`)
    return {
      up: true,
      info: response.data
    }
  } catch (error) {
    return {
      up: false,
      info: { error: error instanceof Error ? error.message : 'Network error' }
    }
  }
}

// Try keys endpoint then vault as a helpful unlock-time probe.
export async function tryKeysThenVault(): Promise<any> {
  // If dev bypass is enabled, don't try the network
  if (env.WALLET_DEV_BYPASS) {
    throw new Error('DEV_BYPASS_ENABLED')
  }

  // If mocking, return a canned vault response so unlock can continue
  if (env.MOCK_CHAIN) {
    return { mock: true, vault: { receipts: [], mocked: true } }
  }

  const base = await getBaseUrl()
  try {
    const resp = await axiosClient.get(`${base}/api/keys`, { timeout: 3000 })
    return resp.data
  } catch (e) {
    // keys unsupported or failed; try vault
    try {
      const resp2 = await axiosClient.get(`${base}/api/wallet/info`, { timeout: 3000 })
      return resp2.data
    } catch (e2) {
      throw new Error('keys_and_vault_unreachable')
    }
  }
}

export async function getSupply(): Promise<{ total: string | number }> {
  // Mock supply for offline/demo
  if (env.MOCK_CHAIN) {
    return { total: 1000000 }
  }

  try {
    const baseUrl = await getBaseUrl();
    const response = await axiosClient.get(`${baseUrl}/api/supply`)
    return response.data
  } catch (error) {
    throw new Error(`Failed to get supply: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function getLatestReceipts(): Promise<any[]> {
  if (env.MOCK_CHAIN) {
    try {
      const raw = localStorage.getItem('mock.receipts')
      return raw ? JSON.parse(raw) : []
    } catch (e) {
      return []
    }
  }

  try {
    const baseUrl = await getBaseUrl();
    const response = await axiosClient.get(`${baseUrl}/api/receipts/latest`)
    return Array.isArray(response.data) ? response.data : []
  } catch (error) {
    throw new Error(`Failed to get receipts: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function getBalance(address: string): Promise<{ LAND: number; GAME: number; CASH: number }> {
  // Mock chain: read balances from localStorage or seed defaults
  if (env.MOCK_CHAIN) {
    try {
      const key = `mock.balances.${address}`
      const raw = localStorage.getItem(key)
      if (raw) return JSON.parse(raw)
      const seed = { LAND: 1, GAME: 250, CASH: 500 }
      localStorage.setItem(key, JSON.stringify(seed))
      return seed
    } catch (e) {
      console.warn('Mock balance read failed', e)
      return { LAND: 0, GAME: 0, CASH: 0 }
    }
  }

  try {
    const baseUrl = await getBaseUrl();
    const response = await axiosClient.get(`${baseUrl}/api/balance/${address}`)
    return {
      LAND: response.data.LAND || 0,
      GAME: response.data.GAME || 0,
      CASH: response.data.CASH || 0
    }
  } catch (error) {
    // For now, return zeros if balance endpoint fails (stub until wired)
    console.warn('Balance fetch failed:', error)
    return { LAND: 0, GAME: 0, CASH: 0 }
  }
}

export interface Transaction {
  token: string
  to: string
  amount: number
  from: string
  nonce: number
}

export interface SignedTransaction {
  tx: Transaction
  sig: string
}

export async function submitTx(payload: SignedTransaction): Promise<{ ok: boolean; txid: string }> {
  // Mock chain: simulate immediate confirmation and persist a mock receipt
  if (env.MOCK_CHAIN) {
    try {
      const tx = payload.tx
      const txid = `mock-${Date.now()}-${Math.floor(Math.random()*10000)}`

      // Update sender balance
      try {
        const fromKey = `mock.balances.${tx.from}`
        const rawFrom = localStorage.getItem(fromKey)
        const fromBal = rawFrom ? JSON.parse(rawFrom) : { LAND: 1, GAME: 250, CASH: 500 }
        if (fromBal[tx.token as keyof typeof fromBal] !== undefined) {
          fromBal[tx.token as keyof typeof fromBal] = Math.max(0, fromBal[tx.token as keyof typeof fromBal] - tx.amount)
        }
        localStorage.setItem(fromKey, JSON.stringify(fromBal))
      } catch (e) {
        console.warn('Failed to update mock sender balance', e)
      }

      // Update recipient balance (best-effort)
      try {
        const toKey = `mock.balances.${tx.to}`
        const rawTo = localStorage.getItem(toKey)
        const toBal = rawTo ? JSON.parse(rawTo) : { LAND: 0, GAME: 0, CASH: 0 }
        if (toBal[tx.token as keyof typeof toBal] !== undefined) {
          toBal[tx.token as keyof typeof toBal] = (toBal[tx.token as keyof typeof toBal] || 0) + tx.amount
        }
        localStorage.setItem(toKey, JSON.stringify(toBal))
      } catch (e) {
        console.warn('Failed to update mock recipient balance', e)
      }

      // Persist mock receipt
      try {
        const raw = localStorage.getItem('mock.receipts')
        const arr = raw ? JSON.parse(raw) : []
        arr.unshift({ txid, from: tx.from, to: tx.to, token: tx.token, amount: tx.amount, time: Date.now(), status: 'confirmed' })
        localStorage.setItem('mock.receipts', JSON.stringify(arr.slice(0, 100)))
      } catch (e) {
        console.warn('Failed to persist mock receipt', e)
      }

      return { ok: true, txid }
    } catch (e) {
      return { ok: false, txid: `error:${e instanceof Error ? e.message : String(e)}` }
    }
  }

  try {
    const baseUrl = await getBaseUrl();
    const response = await axiosClient.post(`${baseUrl}/api/tx/submit`, payload)
    return {
      ok: true,
      txid: response.data.txid || response.data.id || 'unknown'
    }
  } catch (error) {
    return {
      ok: false,
      txid: `error:${error instanceof Error ? error.message : 'Unknown error'}`
    }
  }
}