import { env } from "../../utils/env"

export type CashOrder = {
  id: string;
  buyer_addr: string;
  usd_amount_cents: number;
  cash_amount: number;
  stripe_session_id?: string | null;
  stripe_payment_intent?: string | null;
  payment_status?: string | null;
  status: "created" | "paid" | "minted" | "failed";
  created_at: number;
  updated_at: number;
}

const base = env.MARKET_URL.replace(/\/$/, "")
const ADMIN_TOKEN = (import.meta as any).env.VITE_ADMIN_TOKEN ?? ""

export async function listOrders(
  limit = 50,
  buyer?: string,
  after?: string
): Promise<{ items: CashOrder[]; next_cursor?: string }> {
  const q = new URLSearchParams()
  q.set("limit", String(limit))
  if (buyer) q.set("buyer_addr", buyer)
  if (after) q.set("after", after)
  const r = await fetch(`${base}/admin/cash/orders?${q.toString()}`, {
    headers: ADMIN_TOKEN ? { "X-Admin-Token": ADMIN_TOKEN } : {}
  })
  if (!r.ok) throw new Error(`listOrders ${r.status}`)
  return r.json()
}

export async function getOrder(id: string): Promise<CashOrder> {
  const r = await fetch(`${base}/admin/cash/orders/${id}`, {
    headers: ADMIN_TOKEN ? { "X-Admin-Token": ADMIN_TOKEN } : {}
  })
  if (!r.ok) throw new Error(`getOrder ${r.status}`)
  return r.json()
}

export async function replayMint(id: string): Promise<{ ok: boolean }> {
  const r = await fetch(`${base}/admin/cash/orders/${id}/replay_mint`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(ADMIN_TOKEN ? { "X-Admin-Token": ADMIN_TOKEN } : {})
    },
    body: "{}"
  })
  if (!r.ok) throw new Error(`replayMint ${r.status}`)
  return r.json()
}
