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

// Multi-currency deposit addresses
export async function getDepositAddress(currency: string, userId: string): Promise<{ address: string; currency: string; message?: string }> {
  if (env.MOCK_CHAIN) {
    return {
      currency,
      address: `mock_${currency}_${userId.slice(0, 8)}_deposit`,
      message: 'Mock deposit address for development'
    }
  }

  try {
    return await get(`/api/wallet/deposit/${currency}?user_id=${userId}`)
  } catch (error) {
    throw new Error(`Failed to get deposit address: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

// Multi-currency wallet balances
export async function getWalletBalances(userId: string): Promise<Record<string, { available: number; locked: number }>> {
  if (env.MOCK_CHAIN) {
    return {
      LAND: { available: 1000, locked: 0 },
      BTC: { available: 0.5, locked: 0 },
      BCH: { available: 2.3, locked: 0 },
      DOGE: { available: 1000, locked: 0 }
    }
  }

  try {
    return await get(`/api/wallet/balances?user_id=${userId}`)
  } catch (error) {
    console.warn('Wallet balances fetch failed:', error)
    return {}
  }
}

// Vault status
export interface VaultStatus {
  receipts: any[]
  balances: {
    LAND: { miners: number; dev: number; founders: number }
    BTC: { miners: number; dev: number; founders: number }
    BCH: { miners: number; dev: number; founders: number }
    DOGE: { miners: number; dev: number; founders: number }
  }
  stats?: {
    totalDeposits?: number
    totalWithdrawals?: number
  }
}

// Epoch status from node /vault/epoch
export interface VaultEpochStatus {
  epoch_index: number
  last_payout_height: number
  last_payout_at_ms: number
  vault_balance: string
  fund_balance: string
  treasury_balance: string
  total_weight: string
  due: boolean
  height: number
}

export async function getVaultEpochStatus(): Promise<VaultEpochStatus> {
  if (env.MOCK_CHAIN) {
    return {
      epoch_index: 0,
      last_payout_height: 0,
      last_payout_at_ms: Date.now(),
      vault_balance: '0',
      fund_balance: '0',
      treasury_balance: '0',
      total_weight: '0',
      due: false,
      height: 0,
    };
  }

  try {
    return await get('/vault/epoch');
  } catch (error) {
    throw new Error(`Failed to get vault epoch: ${error instanceof Error ? error.message : 'Network error'}`);
  }
}

export async function getVaultStatus(): Promise<VaultStatus> {
  if (env.MOCK_CHAIN) {
    return {
      receipts: [],
      balances: {
        LAND: { miners: 500000, dev: 300000, founders: 200000 },
        BTC: { miners: 0.5, dev: 0.3, founders: 0.2 },
        BCH: { miners: 10, dev: 6, founders: 4 },
        DOGE: { miners: 50000, dev: 30000, founders: 20000 }
      },
      stats: {
        totalDeposits: 0,
        totalWithdrawals: 0
      }
    }
  }

  try {
    return await get('/api/wallet/info')
  } catch (error) {
    throw new Error(`Failed to get vault status: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

// Exchange API
export interface OrderBook {
  bids: [number, number][]  // [price, size]
  asks: [number, number][]
  chain: string
}

export interface Ticker {
  chain: string
  last: number
  change24h: number
  vol24h: number
  high24h: number
  low24h: number
}

export interface Trade {
  id: string
  ts: number
  price: number
  size: number
  side: 'buy' | 'sell'
  chain: string
}

export interface UserOrder {
  id: string
  chain: string
  side: 'buy' | 'sell'
  price: number | null
  size_total: number
  size_filled: number
  status: 'open' | 'filled' | 'cancelled' | 'partial'
  tif: string
  post_only: boolean
}

export interface OrderRequest {
  owner: string
  chain: string
  side: 'buy' | 'sell'
  price: number
  size: number
  post_only?: boolean
  tif?: 'GTC' | 'IOC' | 'FOK' | 'GTT'
}

export interface OrderResponse {
  ok: boolean
  order_id: string
  trades: Array<{
    id: string
    price: number
    size: number
    buyer: string
    seller: string
  }>
  message: string
}

export async function getExchangeBook(chain: string = 'BTC', depth: number = 50): Promise<OrderBook> {
  if (env.MOCK_CHAIN) {
    return {
      bids: [
        [0.00000042, 1250.50],
        [0.00000041, 2500.00],
        [0.00000040, 5000.00],
        [0.00000039, 10000.00],
        [0.00000038, 7500.00]
      ],
      asks: [
        [0.00000043, 2000.00],
        [0.00000044, 3500.00],
        [0.00000045, 5000.00],
        [0.00000046, 8000.00],
        [0.00000047, 12000.00]
      ],
      chain
    }
  }

  try {
    return await get(`/api/market/exchange/book?chain=${chain}&depth=${depth}`)
  } catch (error) {
    throw new Error(`Failed to get order book: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function getExchangeTicker(chain: string = 'BTC'): Promise<Ticker> {
  if (env.MOCK_CHAIN) {
    return {
      chain,
      last: 0.00000042,
      change24h: 5.2,
      vol24h: 125000,
      high24h: 0.00000045,
      low24h: 0.00000038
    }
  }

  try {
    return await get(`/api/market/exchange/ticker?chain=${chain}`)
  } catch (error) {
    throw new Error(`Failed to get ticker: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function getExchangeTrades(chain: string = 'BTC', limit: number = 50): Promise<Trade[]> {
  if (env.MOCK_CHAIN) {
    return [
      { id: '1', ts: Date.now(), price: 0.00000042, size: 250.50, side: 'buy', chain },
      { id: '2', ts: Date.now() - 17000, price: 0.00000042, size: 500.00, side: 'sell', chain },
      { id: '3', ts: Date.now() - 33000, price: 0.00000043, size: 1000.00, side: 'buy', chain },
      { id: '4', ts: Date.now() - 50000, price: 0.00000041, size: 750.00, side: 'sell', chain },
      { id: '5', ts: Date.now() - 67000, price: 0.00000042, size: 2000.00, side: 'buy', chain }
    ]
  }

  try {
    return await get(`/api/market/exchange/trades?chain=${chain}&limit=${limit}`)
  } catch (error) {
    throw new Error(`Failed to get trades: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function getMyOrders(owner: string, chain: string = 'BTC'): Promise<UserOrder[]> {
  if (env.MOCK_CHAIN) {
    return [
      {
        id: '1',
        chain,
        side: 'buy',
        price: 0.00000040,
        size_total: 5000,
        size_filled: 0,
        status: 'open',
        tif: 'GTC',
        post_only: false
      }
    ]
  }

  try {
    return await get(`/api/market/exchange/my/orders?owner=${encodeURIComponent(owner)}&chain=${chain}`)
  } catch (error) {
    throw new Error(`Failed to get orders: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}

export async function createOrder(request: OrderRequest): Promise<OrderResponse> {
  if (env.MOCK_CHAIN) {
    return {
      ok: true,
      order_id: `mock-order-${Date.now()}`,
      trades: [],
      message: 'Mock order placed on book'
    }
  }

  try {
    return await post('/api/market/exchange/order', request)
  } catch (error) {
    throw new Error(`Failed to create order: ${error instanceof Error ? error.message : 'Network error'}`)
  }
}