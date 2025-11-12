import { useEffect, useState } from 'react'
import { listListings, createCheckout, getListing, simulateWebhook } from '../api/marketApi'

export default function Market() {
  const [listings, setListings] = useState<any[]>([])
  const [loading, setLoading] = useState(false)
  const [devMode] = useState((import.meta as any).env?.VITE_FEATURE_DEV_PANEL === 'true')

  useEffect(() => {
    const load = async () => {
      setLoading(true)
      try {
        const data = await listListings()
        setListings(data)
      } catch (err) {
        console.error('failed to load listings', err)
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [])

  const handleBuy = async (id: string) => {
    try {
      const res = await createCheckout(id)
      if (res.url) {
        window.open(res.url, '_blank')
      } else if (res.session_id) {
        window.open(res.session_id, '_blank')
      }
    } catch (err) {
      console.error('checkout error', err)
      window.pushToast?.('Failed to create checkout', 'error')
    }
  }

  const handleSimulate = async (id: string) => {
    try {
      await simulateWebhook(id)
      // poll until listing settled
      let attempts = 0
      while (attempts < 20) {
        const l = await getListing(id)
        if (l && l.status === 'settled') {
          window.pushToast?.('Listing settled', 'success')
          break
        }
        attempts++
        await new Promise(r => setTimeout(r, 500))
      }
    } catch (err) {
      console.error('simulate failed', err)
    }
  }

  return (
    <div className="market-page">
      <h2>Market</h2>
      {loading && <div>Loading...</div>}
      <div className="listings-grid">
        {listings.map(l => (
          <div key={l.id} className="listing-card">
            <div className="title">{l.title || l.id}</div>
            <div className="price">${(l.price_usd_cents || 0) / 100}</div>
            <button onClick={() => handleBuy(l.id)}>Buy</button>
            {devMode && <button onClick={() => handleSimulate(l.id)}>Mark Paid (DEV)</button>}
          </div>
        ))}
      </div>
    </div>
  )
}
