import { env } from "../../utils/env"

export type Listing = {
  id: string;
  parcel_id?: string;
  seller_addr: string;
  qty_land: number;
  price_amount: number; // minor units
  price_chain: "BTC" | "BCH" | "DOGE";
  pay_to: string;
  status: "open" | "in_mempool" | "confirmed" | "settled" | "cancelled";
  created_at: number;
}

export type CheckoutResp = { session_id: string; url: string } | { url: string }

const base = env.MARKET_URL.replace(/\/$/, "")

export async function listListings(): Promise<Listing[]> {
  const r = await fetch(`${base}/market/land/listings`)
  if (!r.ok) throw new Error(`listListings ${r.status}`)
  return r.json()
}

export async function getListing(id: string): Promise<Listing> {
  const r = await fetch(`${base}/market/land/listings/${id}`)
  if (!r.ok) throw new Error(`getListing ${r.status}`)
  return r.json()
}

export async function createCheckout(parcelId: string, buyerAddr: string, usdCents: number): Promise<CheckoutResp> {
  const r = await fetch(`${base}/cash/buy_intent`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ parcel_id: parcelId, buyer_addr: buyerAddr, usd_amount_cents: usdCents }),
  })
  if (!r.ok) throw new Error(`buy_intent ${r.status}`)
  return r.json()
}

export async function simulateWebhook(sessionId: string, buyerAddr: string, usdCents: number): Promise<void> {
  await fetch(`${base}/cash/simulate_webhook`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      type: "checkout.session.completed",
      data: { object: { id: sessionId, metadata: { buyer_addr: buyerAddr, usd_cents: String(usdCents) } } }
    }),
  })
}
