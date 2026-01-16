import axios from '../lib/axios-wrapper'
import { env } from '../utils/env'

const DEFAULT_BASE = (import.meta as any).env?.VITE_VISION_NODE_URL || 'http://127.0.0.1:7070'

function baseUrl() {
  // If mock or dev bypass is enabled, use same-origin so Vite middleware handles requests
  if (env && (env.MOCK_CHAIN || env.WALLET_DEV_BYPASS)) return ''
  
  // In development with Vite dev server, use empty string to leverage the proxy
  if ((import.meta as any).env.DEV) return ''
  
  return localStorage.getItem('vision.node.url') || DEFAULT_BASE
}

async function withRetry<T>(fn: () => Promise<T>, retries = 1): Promise<T> {
  try {
    return await fn()
  } catch (err) {
    if (retries <= 0) throw err
    // retry once after short delay for 500/timeouts
    await new Promise((res) => setTimeout(res, 300))
    return withRetry(fn, retries - 1)
  }
}

export async function getStatus(): Promise<{ up: boolean; info?: any }> {
  // Try /status first (legacy), then /api/status (current)
  try {
    const res = await axios.get(`${baseUrl()}/status`, { timeout: 3000 })
    return { up: true, info: res.data }
  } catch (err: any) {
    // If /status returns 404, try /api/status
    if (err?.response?.status === 404) {
      try {
        const res = await axios.get(`${baseUrl()}/api/status`, { timeout: 3000 })
        return { up: true, info: res.data }
      } catch (err2: any) {
        // Both failed, return offline
        return { up: false }
      }
    }
    // For other errors, return offline
    return { up: false }
  }
}

export async function getSupply(): Promise<any> {
  return withRetry(async () => {
    const res = await axios.get(`${baseUrl()}/api/supply`, { timeout: 3000 })
    return res.data
  }, 1)
}

export async function getVault(): Promise<any> {
  return withRetry(async () => {
    const res = await axios.get(`${baseUrl()}/api/vault`, { timeout: 3000 })
    return res.data
  }, 1)
}

export async function getLatestReceipts(): Promise<any[]> {
  return withRetry(async () => {
    const res = await axios.get(`${baseUrl()}/api/receipts/latest`, { timeout: 3000 })
    return Array.isArray(res.data) ? res.data : []
  }, 1)
}
