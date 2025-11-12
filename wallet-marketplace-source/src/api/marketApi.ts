import axios from '../lib/axios-wrapper'

const DEFAULT_MARKET = (import.meta as any).env?.VITE_VISION_MARKET_URL || '/api/market'

function baseUrl() {
  return (localStorage.getItem('vision.market.url') || DEFAULT_MARKET)
}

export async function listListings(): Promise<any[]> {
  const res = await axios.get(`${baseUrl()}/land/listings`)
  return Array.isArray(res.data) ? res.data : []
}

export async function getListing(id: string): Promise<any> {
  const res = await axios.get(`${baseUrl()}/land/listings/${id}`)
  return res.data
}

export async function createCheckout(parcelId: string): Promise<{ url: string, session_id?: string }>{
  const res = await axios.post(`${baseUrl()}/cash/buy_intent`, { buyer_addr: parcelId, usd_amount: 1000 })
  // expect { session_url, session_id }
  return res.data
}

// DEV only: trigger simulate webhook
export async function simulateWebhook(id: string): Promise<any> {
  try {
    const res = await axios.post(`${baseUrl()}/cash/simulate_webhook`, { id })
    return res.data
  } catch (err) {
    throw err
  }
}
