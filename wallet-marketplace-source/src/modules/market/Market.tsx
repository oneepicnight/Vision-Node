import React from 'react'
import { Listing, listListings, createCheckout, simulateWebhook, getListing } from './marketApi'
import { env } from '../../utils/env'
import QRCode from 'qrcode'

function short(s: string) { return s.slice(0,6) + '...' + s.slice(-4) }

export default function Market() {
  const [listings, setListings] = React.useState<Listing[]>([])
  const [loading, setLoading] = React.useState(false)
  const [watching, setWatching] = React.useState<Record<string, boolean>>({})
  const [sessionMap, setSessionMap] = React.useState<Record<string, {sessionId?: string, usd?: number, buyer?: string}>>({})

  const dev = env.FEATURE_DEV_PANEL

  React.useEffect(() => {
    let alive = true
    async function load() {
      setLoading(true)
      try {
        const data = await listListings()
        if (alive) setListings(data)
      } catch (e) {
        console.error('load listings', e)
      } finally { setLoading(false) }
    }
    load()
    return () => { alive = false }
  }, [])

  React.useEffect(() => {
    const id = setInterval(async () => {
      const watchingIds = Object.keys(watching).filter(k => watching[k])
      if (watchingIds.length === 0) return
      try {
        for (const id of watchingIds) {
          const l = await getListing(id)
          setListings(prev => prev.map(p => p.id === l.id ? l : p))
          if (l.status === 'settled') setWatching(w => ({...w, [id]: false}))
        }
      } catch (e) { console.error('poll failed', e) }
    }, 2000)
    return () => clearInterval(id)
  }, [watching])

  const handleBuy = async (listing: Listing) => {
    try {
      // For CASH flows we expect to create a Checkout session
      const resp = await createCheckout(listing.parcel_id ?? listing.id, listing.pay_to, 1000)
      const url = (resp as any).url || (resp as any).session_url || (resp as any).session_id
      if (url) {
        window.open(url.toString(), '_blank', 'noopener,noreferrer')
        // store session id for dev simulate
        const sid = (resp as any).session_id || (resp as any).session || undefined
        setSessionMap(m => ({...m, [listing.id]: { sessionId: sid, usd: 1000, buyer: listing.pay_to }}))
        setWatching(w => ({...w, [listing.id]: true}))
      } else {
        window.pushToast?.('Checkout created but no url returned', 'error')
      }
    } catch (e) { console.error('checkout failed', e); window.pushToast?.('Failed to create checkout', 'error') }
  }

  async function toDataURL(text: string): Promise<string> {
    try { return await QRCode.toDataURL(text, { margin: 1, width: 192 }); }
    catch { return "" }
  }

  const handleShowQR = async (listing: Listing) => {
    try {
      const data = await toDataURL(listing.pay_to)
      // open small window with image
      const w = window.open('', '_blank', 'noopener,noreferrer')
      if (w) {
        w.document.write(`<img src="${data}" alt="QR" style="width:256px;height:256px;border:1px solid #ddd;border-radius:8px" />`)
      }
    } catch (e) { console.error('qr failed', e) }
  }

  const handleCopy = async (listing: Listing) => {
    try {
      await navigator.clipboard.writeText(listing.pay_to)
      window.pushToast?.('Copied!', 'success')
    } catch (e) { console.error('copy failed', e); window.pushToast?.('Copy failed', 'error') }
  }

  const handleSimulate = async (listing: Listing) => {
    try {
      const info = sessionMap[listing.id]
      const sid = info?.sessionId ?? `cs_${listing.id}`
      await simulateWebhook(sid, info?.buyer ?? listing.pay_to, info?.usd ?? 1000)
      // start watching
      setWatching(w => ({...w, [listing.id]: true}))
    } catch (e) { console.error('simulate', e) }
  }

  return (
    <div className="p-4">
      <h2 className="text-2xl font-semibold mb-4">Market</h2>
      {loading && <div>Loading…</div>}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {listings.map(l => (
          <div key={l.id} className="p-4 bg-white rounded shadow">
            <div className="text-sm text-gray-500">{l.created_at ? new Date(l.created_at*1000).toLocaleString() : ''}</div>
            <div className="font-medium mt-1">{l.parcel_id ?? l.id}</div>
            <div className="text-xs text-gray-600">Seller: {short(l.seller_addr)}</div>
            <div className="mt-2">Qty: <strong>{l.qty_land}</strong></div>
            <div className="mt-1">Price: <strong>{(l.price_amount/100).toFixed(2)} {l.price_chain}</strong></div>
            <div className="mt-3 flex gap-2">
              <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={() => handleBuy(l)}>Buy</button>
              <button className="px-3 py-1 bg-gray-200 text-gray-800 rounded" onClick={() => handleShowQR(l)}>Show QR</button>
              <button className="px-3 py-1 bg-gray-200 text-gray-800 rounded" onClick={() => handleCopy(l)}>Copy address</button>
              {dev && <button className="px-3 py-1 bg-green-600 text-white rounded" onClick={() => handleSimulate(l)}>Mark Paid (DEV)</button>}
              <div className="ml-auto text-sm">Status: <strong>{l.status}</strong></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
